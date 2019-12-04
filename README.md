Regulators is a hackathon project to create a simple, pluggable system to create approval workflows in CD/CI pipelines, provide checks in a deployment system or for any other use. These workflow tasks could include locks, code review verification, safety and security checks, alarm validation, etc.

## Development

### Prequisites

You must have Docker and Rust installed.

### Basic Commands

```
# Build
$ cargo build
# Run server
$ cargo run
```

## Docker

```
docker build -t regulators .
docker run -it --rm --publish 9000:8000 --name regulators regulators
```

## Details

### POST /regulate API details

The JSON data for `POST /regulate` should be something like this:

```
{
    "regulators": [{
        "name": "regulators-acquire-lock",
        "context": {
            "lock_key": "foo::bar"
        }
    }, {
        "name": "regulators-code-review-verification",
        "context": {
            "repository": "https://github.com/mmerkes/Regulators",
            "required_approvers": 1
        }
    }, {
        "name": "MyCustomRegulator",
        "context": {
            "foo": "bar"
        }
    }]
}
```

### Calling Lambdas

The `/regulate` API calls Lambdas asynchronously and waits for them to complete their tasks via the `PUT /workflows/:workflow/tasks/:task` API. It will send them content in the following format:

```
{
    "workflow_id": "some-id",
    "task_id": "some-task",
    "content": {
        "keys": "exactly like the customer called with"
    }
}
```

### GET /workflows/:workflow

Gets the workflow information by workflow ID.

### GET /workflows/:workflow/tasks/:task

Gets the task information by workflow and task ID.

### PUT /workflows/:workflow/tasks/:task

This API is used to complete tasks and trigger next events once the last task is completed. The JSON data for the `PUT /workflows/:workflow/tasks/:task` should be something like this:

```
{
    "status": "Succeeded"
}
```

Possible statuses are `Succeeded` and `Failed`.
