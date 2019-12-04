#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate serde_derive;
extern crate rusoto_core;
extern crate rusoto_dynamodb;
extern crate serde;
extern crate serde_dynamodb;

use rocket::State;
use rocket_contrib::json::Json;
use rusoto_core::Region;
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, AttributeValue, GetItemInput, PutItemInput};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

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

#[post("/regulate", data = "<data>")]
fn regulate(data: Json<RegulateData>, ddb: State<DynamoDbClient>) -> Json<RegulateResponse> {
    let regulators = data.into_inner().regulators;

    let workflow_id = Uuid::new_v4();
    let workflow = Workflow {
        id: workflow_id.to_string(),
        status: "InProgress".to_owned(),
    };

    // This block could be improved.
    match serde_dynamodb::to_hashmap(&workflow) {
        Ok(workflow_ddb) => {
            let mut put_item_input: PutItemInput = Default::default();
            put_item_input.item = workflow_ddb;
            put_item_input.table_name = "workflows".to_owned();
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

    Json(RegulateResponse {
        id: workflow_id.to_string(),
    })
}

#[get("/workflows/<workflow>")]
fn get_workflow(workflow: String) -> String {
	"Returns workflow information".to_string()
}

#[put("/workflows/<workflow>/tasks/<task>")]
fn update_task(workflow: String, task: String) {
}

#[get("/workflows/<workflow>/tasks/<task>")]
fn get_task(workflow: String, task: String, ddb: State<DynamoDbClient>) -> Option<Json<WorkflowTask>> {
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
    return match ddb.get_item(get_item_input).sync() {
        Ok(output) => {
            match serde_dynamodb::from_hashmap(output.item.unwrap()) {
                Ok(workflow_task) => {
                    Some(Json(workflow_task))
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
    };
}

fn main() {
    rocket::ignite()
        .manage(DynamoDbClient::new(Region::UsEast1))
        .mount("/", routes![regulate, get_workflow,
                           update_task, get_task])
        .launch();
}
