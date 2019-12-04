#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate serde_derive;
extern crate rusoto_core;
extern crate rusoto_dynamodb;
extern crate serde;
extern crate serde_dynamodb;

use rocket_contrib::json::Json;
use rusoto_core::Region;
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, PutItemInput};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
struct Workflow {
    id: String,
    status: String,
}

#[derive(Serialize)]
struct RegulateResponse {
    id: String
}

#[post("/regulate")]
fn regulate() -> Json<RegulateResponse> {
    let workflow_id = Uuid::new_v4();
    let workflow = Workflow {
        id: workflow_id.to_string(),
        status: "InProgress".to_owned(),
    };
	let client = DynamoDbClient::new(Region::UsEast1);

    match serde_dynamodb::to_hashmap(&workflow) {
        Ok(workflow_ddb) => {
            let mut put_item_input: PutItemInput = Default::default();
            put_item_input.item = workflow_ddb;
            put_item_input.table_name = "workflows".to_owned();
            match client.put_item(put_item_input).sync() {
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
fn get_task(workflow: String, task: String) -> String {
	"Returns task information".to_string()
}

fn main() {
    rocket::ignite().mount("/", routes![regulate, get_workflow,
                           update_task, get_task]).launch();
}
