#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate serde_derive;
extern crate rusoto_core;
extern crate rusoto_dynamodb;
extern crate serde;
extern crate serde_dynamodb;

use bytes::Bytes;
use rocket::State;
use rocket::http::Status;
use rocket_contrib::json::Json;
use rusoto_core::{RusotoError, Region};
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, AttributeValue, GetItemInput, GetItemError, PutItemInput, PutItemError, QueryInput, QueryError};
use rusoto_lambda::{Lambda, LambdaClient, InvokeAsyncRequest, InvokeAsyncError};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug)]
enum RegulatorsError {
    GetItemError(RusotoError<GetItemError>),
    PutItemError(RusotoError<PutItemError>),
    QueryError(RusotoError<QueryError>),
    InvokeAsyncError(RusotoError<InvokeAsyncError>),
    SerdeError(serde_dynamodb::Error),
}

impl From<serde_dynamodb::Error> for RegulatorsError {
    fn from(se: serde_dynamodb::Error) -> Self {
        RegulatorsError::SerdeError(se)
    }
}

impl From<RusotoError<PutItemError>> for RegulatorsError {
    fn from(re: RusotoError<PutItemError>) -> Self {
        RegulatorsError::PutItemError(re)
    }
}

impl From<RusotoError<QueryError>> for RegulatorsError {
    fn from(re: RusotoError<QueryError>) -> Self {
        RegulatorsError::QueryError(re)
    }
}

impl From<RusotoError<GetItemError>> for RegulatorsError {
    fn from(re: RusotoError<GetItemError>) -> Self {
        RegulatorsError::GetItemError(re)
    }
}

impl From<RusotoError<InvokeAsyncError>> for RegulatorsError {
    fn from(re: RusotoError<InvokeAsyncError>) -> Self {
        RegulatorsError::InvokeAsyncError(re)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Workflow {
    id: String,
    status: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Regulator {
    name: String,
    context: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct WorkflowTask {
    id: String,
    workflow_id: String,
    status: String,
    regulator: Regulator,
}

#[derive(Serialize)]
struct RegulateResponse {
    id: String
}

#[derive(Deserialize)]
struct RegulateData {
    regulators: Vec<Regulator>,
}

#[derive(Deserialize)]
struct PutTaskData {
    status: String,
}

#[derive(Serialize)]
struct GetWorkflowResponse {
    id: String,
    status: String,
    tasks: Vec<WorkflowTask>,
}

fn _update_workflow(workflow: Workflow, ddb: &State<DynamoDbClient>) -> Result<(), RegulatorsError> {
    match serde_dynamodb::to_hashmap(&workflow) {
        Ok(workflow_ddb) => {
            let mut put_item_input: PutItemInput = Default::default();
            put_item_input.item = workflow_ddb;
            put_item_input.table_name = "workflows".to_owned();
            match ddb.put_item(put_item_input).sync() {
                Ok(_output) => {
                    Ok(())
                },
                Err(err) => {
                    println!("Error: {:?}", err);
                    Err(RegulatorsError::PutItemError(err))
                }
            }
        },
        Err(err) => {
            Err(RegulatorsError::SerdeError(err))
        }
    }
}

#[derive(Serialize)]
struct RegulatorInvokeArgs {
    workflow_id: String,
    task_id: String,
    context: HashMap<String, Value>,
}

fn _invoke_lambda(task: &WorkflowTask, lambda: &State<LambdaClient>) -> Result<(), RegulatorsError> {
    let args = Bytes::from(serde_json::to_string(&RegulatorInvokeArgs {
        workflow_id: task.workflow_id.clone(),
        task_id: task.id.clone(),
        context: task.regulator.context.clone(),
    }).unwrap());

    let request = InvokeAsyncRequest {
        function_name: task.regulator.name.clone(),
        invoke_args: args,
        ..Default::default()
    };

    match lambda.invoke_async(request).sync() {
        Ok(_output) => Ok(()),
        Err(err) => Err(RegulatorsError::InvokeAsyncError(err))
    }
}

#[get("/")]
fn index() -> &'static str {
    "Regulators!"
}

#[post("/regulate", data = "<data>")]
fn regulate(data: Json<RegulateData>, ddb: State<DynamoDbClient>, lambda: State<LambdaClient>) -> Json<RegulateResponse> {
    let regulators = data.into_inner().regulators;

    let workflow_id = Uuid::new_v4();
    let workflow = Workflow {
        id: workflow_id.to_string(),
        status: "InProgress".to_owned(),
    };

    match _update_workflow(workflow, &ddb) {
        Ok(()) => (),
        Err(err) => {
            println!("{:?}", err);
            return Json(RegulateResponse {
                id: "".to_string(),
            });
        }
    }

    let workflow_tasks: Vec<WorkflowTask> = regulators.iter().map(|regulator| {
        WorkflowTask {
            id: Uuid::new_v4().to_string(),
            workflow_id: workflow_id.to_string(),
            status: "InProgress".to_owned(),
            // Assuredly, this can be done more elegantly.
            regulator: Regulator {
                name: regulator.name.clone(),
                context: regulator.context.clone(),
            },
        }
    }).collect();

    // Batch into a single DDB call.
    for task in &workflow_tasks {
        match serde_dynamodb::to_hashmap(&task) {
            Ok(task_ddb) => {
                let mut put_item_input: PutItemInput = Default::default();
                put_item_input.item = task_ddb;
                put_item_input.table_name = "tasks".to_owned();
                match ddb.put_item(put_item_input).sync() {
                    Ok(_output) => {
                    }
                    Err(error) => {
                        println!("Error: {:?}", error);
                        return Json(RegulateResponse {
                            id: "".to_string(),
                        });
                    }
                }
            },
            Err(err) => {
                println!("{:?}", err);
                return Json(RegulateResponse {
                    id: "".to_string(),
                });
            }
        }
    }

    // Invoking the lambdas as a second step so that we know we've persisted them first.
    for task in &workflow_tasks {
        match _invoke_lambda(task, &lambda) {
            Ok(_response) => (),
            Err(err) => {
                println!("{:?}", err);
                return Json(RegulateResponse {
                    id: "".to_string(),
                });
            }
        }
    }

    Json(RegulateResponse {
        id: workflow_id.to_string(),
    })
}

fn _get_workflow(workflow: String, ddb: &State<DynamoDbClient>) -> Result<Option<Workflow>, RegulatorsError> {
    let mut key = HashMap::new();
    let mut primary_key: AttributeValue = Default::default();
    primary_key.s = Some(workflow);
    key.insert("id".to_string(), primary_key);
    let mut get_item_input: GetItemInput = Default::default();
    get_item_input.key = key;
    get_item_input.table_name = "workflows".to_string();
    match ddb.get_item(get_item_input).sync() {
        Ok(output) => {
            if output.item.is_none() {
                return Ok(None);
            }

            match serde_dynamodb::from_hashmap(output.item.unwrap()) {
                Ok(workflow) => {
                    Ok(Some(workflow))
                },
                Err(err) => {
                    Err(RegulatorsError::SerdeError(err))
                }
            }
        }
        Err(err) => {
            Err(RegulatorsError::GetItemError(err))
        }
    }
}

fn _get_tasks(workflow: String, ddb: &State<DynamoDbClient>) -> Result<Vec<WorkflowTask>, RegulatorsError> {
    let mut query = HashMap::new();
    query.insert(String::from(":workflow_id"), AttributeValue {
        s: Some(String::from(workflow)),
        ..Default::default()
    });

    let tasks = ddb
        .query(QueryInput {
            table_name: String::from("tasks"),
            key_condition_expression: Some(String::from("workflow_id = :workflow_id")),
            expression_attribute_values: Some(query),
            ..Default::default()
        })
        .sync()
        .unwrap()
        .items
        .unwrap_or_else(|| vec![])
        .into_iter()
        .map(|item| serde_dynamodb::from_hashmap(item).unwrap())
        .collect();

    Ok(tasks)
}

#[get("/workflows/<workflow>")]
fn get_workflow(workflow: String, ddb: State<DynamoDbClient>) -> Option<Json<GetWorkflowResponse>> {
    match _get_workflow(workflow.clone(), &ddb) {
        Ok(workflow_option) => {
            if workflow_option.is_none() {
                return None;
            }

            let workflow_ddb = workflow_option.unwrap();

            match _get_tasks(workflow.clone(), &ddb) {
                Ok(tasks) => {
                    Some(Json(GetWorkflowResponse {
                        id: workflow.clone(),
                        status: workflow_ddb.status.clone(),
                        tasks: tasks,
                    }))
                },
                Err(err) => {
                    println!("{:?}", err);
                    None
                }
            }
        },
        Err(err) => {
            println!("{:?}", err);
            None
        }
    }
}

fn _get_task(workflow: String, task: String, ddb: &State<DynamoDbClient>) -> Option<WorkflowTask> {
    let mut key = HashMap::new();
    let mut primary_key: AttributeValue = Default::default();
    primary_key.s = Some(workflow);
    let mut sort_key: AttributeValue = Default::default();
    sort_key.s = Some(task);
    key.insert("workflow_id".to_string(), primary_key);
    key.insert("id".to_string(), sort_key);
    let mut get_item_input: GetItemInput = Default::default();
    get_item_input.key = key;
    get_item_input.table_name = "tasks".to_string();
    match ddb.get_item(get_item_input).sync() {
        Ok(output) => {
            if output.item.is_none() {
                return None;
            }

            match serde_dynamodb::from_hashmap(output.item.unwrap()) {
                Ok(workflow_task) => {
                    Some(workflow_task)
                },
                Err(err) => {
                    println!("{:?}", err);
                    None
                }
            }
        }
        Err(error) => {
            println!("Error: {:?}", error);
            None
        }
    }
}

fn _update_task_status(mut task: WorkflowTask, status: String, ddb: &State<DynamoDbClient>) -> Result<(), RegulatorsError> {
    task.status = status;
    match serde_dynamodb::to_hashmap(&task) {
        Ok(task_ddb) => {
            let mut put_item_input: PutItemInput = Default::default();
            put_item_input.item = task_ddb;
            put_item_input.table_name = "tasks".to_owned();
            match ddb.put_item(put_item_input).sync() {
                Ok(_output) => {
                    Ok(())
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    Err(RegulatorsError::PutItemError(err))
                }
            }
        },
        Err(err) => {
            Err(RegulatorsError::SerdeError(err))
        }
    }
}

fn _workflow_has_pending_tasks(workflow: String, ddb: &State<DynamoDbClient>) -> Result<bool, RegulatorsError> {
    match _get_tasks(workflow.clone(), ddb) {
        Ok(tasks) => {
            let has_pending = tasks.into_iter()
                .any(|task: WorkflowTask| task.status != "Failed" && task.status != "Succeeded");

            Ok(has_pending)
        },
        Err(err) => Err(err)
    }
}

#[put("/workflows/<workflow>/tasks/<task>", data = "<data>")]
fn update_task(workflow: String, task: String, data: Json<PutTaskData>, ddb: State<DynamoDbClient>) -> Status {
    match _get_task(workflow.clone(), task, &ddb) {
        Some(workflow_task) => {
            match _update_task_status(workflow_task, data.status.clone(), &ddb) {
                Ok(_output) => (),
                Err(err) => {
                    println!("Error: {:?}", err);
                    return Status::BadRequest;
                }
            }
        },
        None => {
            return Status::NotFound;
        }
    }

    if data.status == "Failed" {
        match _get_workflow(workflow.clone(), &ddb) {
            Ok(workflow_opt) => {
                if workflow_opt.is_none() {
                    println!("Workflow not found for {}", workflow.clone());
                    return Status::NotFound;
                }

                let mut workflow_ddb = workflow_opt.unwrap();
                workflow_ddb.status = data.status.clone();
                workflow_ddb.status = data.status.clone();
                match _update_workflow(workflow_ddb, &ddb) {
                    Ok(()) => {
                        return Status::Accepted;
                    },
                    Err(err) => {
                        println!("Error: {:?}", err);
                        return Status::BadRequest;
                    }
                }
            },
            Err(err) => {
                println!("Error: {:?}", err);
                return Status::BadRequest;
            }
        }
    }

    if data.status == "Succeeded" {
        match _get_workflow(workflow.clone(), &ddb) {
            Ok(workflow_opt) => {
                if workflow_opt.is_none() {
                    println!("Workflow not found for {}", workflow.clone());
                    return Status::NotFound;
                }

                let mut workflow_ddb = workflow_opt.unwrap();

                if workflow_ddb.status == "Failed" || workflow_ddb.status == "Succeeded" {
                    println!("Workflow status is in a final status for workflow {}. Doing nothing.", workflow.clone());
                    return Status::Accepted;
                }


                match _workflow_has_pending_tasks(workflow.clone(), &ddb) {
                    Ok(has_pending) => {
                        if !has_pending {
                            workflow_ddb.status = data.status.clone();
                            workflow_ddb.status = data.status.clone();
                            match _update_workflow(workflow_ddb, &ddb) {
                                Ok(()) => (),
                                Err(err) => {
                                    println!("Error: {:?}", err);
                                    return Status::BadRequest;
                                }
                            }
                        }

                        return Status::Accepted;
                    },
                    Err(err) => {
                        println!("Error: {:?}", err);
                        return Status::BadRequest;
                    }
                }
            },
            Err(err) => {
                println!("Error: {:?}", err);
                return Status::BadRequest;
            }
        }
    }

    Status::BadRequest
}

#[get("/workflows/<workflow>/tasks/<task>")]
fn get_task(workflow: String, task: String, ddb: State<DynamoDbClient>) -> Option<Json<WorkflowTask>> {
    match _get_task(workflow, task, &ddb) {
        Some(task) => {
            Some(Json(task))
        },
        None => None
    }
}

fn main() {
    rocket::ignite()
        .manage(DynamoDbClient::new(Region::UsEast1))
        .manage(LambdaClient::new(Region::UsEast1))
        .mount("/", routes![index, regulate, get_workflow, update_task, get_task])
        .launch();
}
