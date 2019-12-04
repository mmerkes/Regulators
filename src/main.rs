#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate serde_derive;
extern crate rusoto_core;
extern crate rusoto_dynamodb;
extern crate serde;
extern crate serde_dynamodb;

use rocket::State;
use rocket::http::Status;
use rocket_contrib::json::Json;
use rusoto_core::{RusotoError, Region};
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, AttributeValue, GetItemInput, GetItemError, PutItemInput, PutItemError};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug)]
enum RegulatorsError {
    GetItemError(RusotoError<GetItemError>),
    PutItemError(RusotoError<PutItemError>),
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

impl From<RusotoError<GetItemError>> for RegulatorsError {
    fn from(re: RusotoError<GetItemError>) -> Self {
        RegulatorsError::GetItemError(re)
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

#[post("/regulate", data = "<data>")]
fn regulate(data: Json<RegulateData>, ddb: State<DynamoDbClient>) -> Json<RegulateResponse> {
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
            })
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
                    }
                }
            },
            Err(err) => {
                println!("{:?}", err);
            }
        }
    }

    // TODO: Call Lambdas

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

#[get("/workflows/<workflow>")]
fn get_workflow(workflow: String, ddb: State<DynamoDbClient>) -> Option<Json<Workflow>> {
    match _get_workflow(workflow.clone(), &ddb) {
        Ok(workflow_option) => {
            match workflow_option {
                Some(workflow) => Some(Json(workflow)),
                None => None
            }
        },
        Err(err) => None
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

#[put("/workflows/<workflow>/tasks/<task>", data = "<data>")]
fn update_task(workflow: String, task: String, data: Json<PutTaskData>, ddb: State<DynamoDbClient>) -> Status {
    match _get_task(workflow, task, &ddb) {
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

    // TODO: If failed, fail the workflow

    // TODO: If succeeded, check if all tasks succeeded, and succeed workflow if all succeeded

    Status::Accepted
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
        .mount("/", routes![regulate, get_workflow,
                           update_task, get_task])
        .launch();
}
