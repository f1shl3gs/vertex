use value::{value, Value};
use vtl::{compile, Context, TargetValue};

fn main() {
    let script = r#"

    url, err = parse_url("https://example.io?foo=bar")
    url.fragment = "blah"
    log("host:", url)
    if url.query.foo == "bar" {
        log("foo is bar")
    } else {
        log("foo is not bar")
    }

    # some function might return error
    shell, err = get_env("SHELL")
    log("shell:", shell, err)
    if err == null {
        log("get shell ok")
    }

    # like rust, return error now
    shell = get_env("SHELL")?
    log("shell:", shell)

    a = -1
    b = (1 + 2 - 3) * 2 / 2
    log("a", a, b)

    if .index + 10 == 15 {
        log("15")
    }

    for index, item in .array {
        log("array:", index, item)
    }

    for key, value in .map {
        log("map:", key, value)
    }

    log("msg:", .msg)

    hostname, err = get_hostname()
    log("hostname:", hostname, err)

    # container
    arr = [1, 2, 3, true || false, true && false, 1 + 2 + 5 ]
    log("array:", arr)
    log("array[2]", arr[2])

    map = {
        str: "bar",
        int: 1,
        map: {
            key: "value"
        },
        array: [1, 2, 3]
    }
    log("map:", map)

    .msg = "{\"foo\": 1}"
    .msg = parse_json(.msg)?
    log("msg:", .msg)

    for index, item in [1, 2, 3] {
        if index == 1 {
            continue
        }

        if item == 3 {
            break
        }

        log("array index:", index)
    }

    "#;

    let program = compile(script).unwrap();
    let mut variables = program.variables.clone();

    // build your own context
    let mut cx = Context {
        target: &mut TargetValue {
            metadata: Value::Object(Default::default()),
            value: value!({
                "msg": "foobar",
                "index": 5,
                "array": [
                    1,
                    2,
                    3,
                    {
                        "ak": "av"
                    }
                ],
                "map": {
                    "k1": "k2"
                },
            }),
        },
        variables: &mut variables,
    };

    if let Err(err) = program.resolve(&mut cx) {
        panic!("{}", err);
    }
}
