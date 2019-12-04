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
        "name": "Regulators_AcquireLock",
        "context": {
            "lock_key": "foo::bar"
        }
    }, {
        "name": "Regulators_CodeReviewVerification",
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
