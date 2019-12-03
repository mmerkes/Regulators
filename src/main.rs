#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

#[post("/regulate")]
fn regulate() {
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
