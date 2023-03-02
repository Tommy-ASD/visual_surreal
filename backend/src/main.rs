use std::{
    fs::{create_dir_all, read_to_string, File},
    io::Write,
};

use async_recursion::async_recursion;
use log::debug;
use serde_json::{Map, Value};

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Cow;
use surrealdb::sql;
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;

#[derive(Serialize, Deserialize)]
struct Name {
    first: Cow<'static, str>,
    last: Cow<'static, str>,
}

#[derive(Serialize, Deserialize)]
struct Person {
    #[serde(skip_serializing)]
    id: Option<String>,
    title: Cow<'static, str>,
    name: Name,
    marketing: bool,
}

#[tokio::main]
async fn main() -> surrealdb::Result<()> {
    let db = Surreal::new::<Ws>("localhost:8000").await?;

    // Signin as a namespace, database, or root user
    db.signin(Root {
        username: "root",
        password: "root",
    })
    .await?;

    // Select a specific namespace and database
    db.use_ns("namespace").use_db("database").await?;

    // Create a new person with a random ID
    let tobie: Person = db
        .create("person")
        .content(Person {
            id: None,
            title: "Founder & CEO".into(),
            name: Name {
                first: "Tobie".into(),
                last: "Morgan Hitchcock".into(),
            },
            marketing: true,
        })
        .await?;

    assert!(tobie.id.is_some());

    // Create a new person with a specific ID
    let mut jaime: Person = db
        .create(("person", "jaime"))
        .content(Person {
            id: None,
            title: "Founder & COO".into(),
            name: Name {
                first: "Jaime".into(),
                last: "Morgan Hitchcock".into(),
            },
            marketing: false,
        })
        .await?;

    assert_eq!(jaime.id.unwrap(), "person:jaime");

    // Update a person record with a specific ID
    jaime = db
        .update(("person", "jaime"))
        .merge(json!({ "marketing": true }))
        .await?;

    assert!(jaime.marketing);

    // Select all people records
    let people: Vec<Person> = db.select("person").await?;

    assert!(!people.is_empty());

    // Perform a custom advanced query
    let sql = r#"
        SELECT marketing, count()
        FROM type::table($table)
        GROUP BY marketing
    "#;

    let groups = db.query(sql)
        .bind(("table", "person"))
        .await?;

    dbg!(groups);

    // Delete all people upto but not including Jaime
    db.delete("person").range(.."jaime").await?;

    // Delete all people
    db.delete("person").await?;

    Ok(())
}
c
/*
This function sends a body to the database

Parameters:
    - body: String - the body to send to the database

Returns:
    None

Possible problems:
    - if the database is not available, the function will endlessly retry (Acceptable?)
    - if the env file is not set up correctly, the build will fail (Acceptable)
    Any further errors will be written to errors.txt

Possible improvements:
    - change header values from type String to type &str to save memory and time (minimal impact)

Calls:
    - upload_body_to_db - this function calls itself if the db is not available
        but with the #[async_recursion] attribute, it will not overflow the stack
*/
#[async_recursion]
pub async fn upload_body_to_db(body: String) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut url: String = "http://localhost:8000/sql".to_string();
    let mut user: String = "root".to_string();
    let mut pass: String = "root".to_string();
    let mut ns: String = "test".to_string();
    let mut db: String = "test".to_string();
    for (key, value) in std::env::vars() {
        match key.as_str() {
            "DB_URL" => url = value,
            "DB_USER" => user = value,
            "DB_PASS" => pass = value,
            "DB_NS" => ns = value,
            "DB_DB" => db = value,
            _ => {}
        }
    }
    let res = match client
        .post(url)
        .basic_auth(user, Some(pass))
        .header("NS", ns)
        .header("DB", db)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .body(body.clone())
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => {
            let secs = 10;
            for _ in 0..secs {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            return upload_body_to_db(body).await;
        }
    };
    let result;
    if res.status() != 200 {
        add_to_errors_file(&format!("{:?}", &res.status()));
        add_to_errors_file(&format!("Error uploading to database: {:#?}", &res));
        result = format!("{:#?}", &res.text().await);
        add_to_errors_file(&result);
        add_to_errors_file(&format!("{:?}", body));
        add_to_errors_file(&format!("{}", body));
    } else {
        result = res.text().await?;
    }
    Ok(result)
}

/*
This function uploads a body to the database and extracts the result at the given index

Parameters:
    - query: &str - the query to send to the database
    - index: usize - the index of the result to extract

Returns:
    - Ok(Vec<Value>) - the result at the given index
    - Err(String) - the error message

Possible problems:
    - if the database is not available, the function will endlessly retry (Acceptable?)
    - if the env file is not set up correctly, the build will fail (Acceptable)
    Any further errors will be written to errors.txt

Possible improvements:
    - change header values from type String to type &str to save memory and time (minimal impact)

Calls:
    - upload_body_to_db - this function calls itself if the db is not available
        but with the #[async_recursion] attribute, it will not overflow the stack
    - extract_nested_json! - this function extracts the result at the given index
 */
pub async fn upload_and_extract_from_db(
    query: &str,
    index: usize,
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    debug!("Uploading to DB: {}", query);
    let response: Value = match upload_body_to_db(query.to_string()).await {
        Ok(res) => match serde_json::from_str(&res) {
            Ok(res) => res,
            Err(e) => {
                // database returned invalid json, return 500
                let err = format!("DB returned invalid JSON: {}", e);
                return Err(err.into());
            }
        },
        Err(e) => {
            let err = format!("Uploading to DB failed: {}", e);
            return Err(err.into());
        }
    };
    let response_array = match response.as_array() {
        Some(res) => res,
        None => {
            let err = format!(
                "Database returned unexptected JSON: top-level is not an array ({response})."
            );
            return Err(err.into());
        }
    };
    let response_object = match response_array.get(index) {
        Some(res) => res,
        None => {
            let err = format!("Database returned unexptected JSON: array is empty ({response}).");
            return Err(err.into());
        }
    };
    let result = match response_object.get("result") {
        Some(res) => res,
        None => {
            let err = format!("Database returned unexptected JSON: object does not contain 'result' key ({response}).");
            return Err(err.into());
        }
    };
    let result_array = match result.as_array() {
        Some(res) => res,
        None => {
            let err = format!(
                "Database returned unexptected JSON: 'result' is not an array ({response})."
            );
            return Err(err.into());
        }
    };
    let test_result_array = extract_nested_json!(response, Vec<Value>, index, "result")();
    match test_result_array {
        Ok(t) => {
            if &t != result_array {
                dbg!("T {} != result_array {}", t, result_array);
            }
        }
        Err(e) => {
            let err = format!("Error when extracting nested JSON: {}", e);
            dbg!(err);
        }
    }
    Ok(result_array.clone())
}
pub fn add_to_errors_file(e: &str) {
    let mut prev_content = std::fs::read_to_string("errors.txt").unwrap_or(String::new());
    prev_content.push_str(&format!("{e}\n"));
    match std::fs::write("errors.txt", prev_content) {
        Ok(_) => (),
        Err(e) => println!("Error writing to errors.txt: {}", e),
    }
}
/*
This function is used to detect nested json values.
It takes a json object, and returns the keys of all json values that are nested in the object.
It does this recursively, so if there are nested objects in the nested objects, it will return the keys of those as well.
This is returned as a vector of strings.
*/
pub fn detect_nested_json(json: &Map<String, Value>) -> Vec<String> {
    let mut keys = Vec::new();
    for (key, value) in json {
        match value {
            Value::Object(nested_obj) => {
                if !nested_obj.is_empty() {
                    let nested_keys = detect_nested_json(nested_obj);
                    for nested_key in nested_keys {
                        keys.push(format!("{}.{}", key, nested_key));
                    }
                }
            }
            _ => {
                keys.push(key.to_string());
            }
        }
    }
    keys
}
#[macro_export]
macro_rules! extract_nested_json {
    ($json:expr, $ret_type:ty, $key:expr) => {
        || {
            let j = $json[$key].clone();
            let parsed_to_type: $ret_type = match serde_json::from_value(j) {
                Ok(v) => v,
                Err(e) => return Err(
                    format!(
                        "Error when getting key {key} from json {json} with expected type {type}: {error}", key = $key, json = $json, type = stringify!($ret_type), error = e
                    )
                ),
            };
            Ok(parsed_to_type)
        }
    };
    ($json:expr, $ret_type:ty, $key:expr, $($keys:expr),*) => {
        extract_nested_json!($json[$key], $ret_type, $($keys),*)
    };
}

/*
This function is simply the input() function from Python, as getting input is a bit annoying in Rust.
*/
pub fn input() -> String {
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(_) => {}
        Err(e) => {
            println!("Error reading line: {}", e);
            return input();
        }
    }
    line.trim().to_string()
}

/*
This function is used to split the roller.json file into multiple files.
The reason for this is that the file is too large to be uploaded to Surreal.
It should work with any JSON array, but it's only been tested with roller.json.

Parameters:
    filepath: &str - the path to the JSON file
    split_size: usize - the size of the split files

Returns:
    None, as it only writes to files

Possible problems:
    - if the file can not be read, the function will exit early
    - if the file is malformed, the function will exit early
    - if the JSON file is not an array, the function will exit early
    - if writing to the files fails, it will not work
    - if the file is too large and the host machine doesn't have enough memory,
      i am not sure what will happen (probably a panic)
    - if the file is too large and the host machine doesn't have enough disk space,
      i am also not sure what will happen (probably a panic)

Possible improvements:
    - reduce code repetition
    - general code cleanup

Calls:
    - none
*/
pub fn split_array_from_json_file(filepath: &str, split_size: usize) {
    let str = match read_to_string(filepath) {
        Ok(s) => s,
        Err(e) => {
            println!("Error when reading roller JSON: {}", e);
            return;
        }
    };
    let parsed: serde_json::Value = match serde_json::from_str(&str) {
        Ok(p) => p,
        Err(e) => {
            println!("Error when parsing roller JSON: {}", e);
            return;
        }
    };
    let parsed = match parsed.as_array() {
        Some(p) => p,
        None => {
            println!("Error when parsing roller JSON: not an array");
            return;
        }
    };
    let folder_path = filepath.split(".").collect::<Vec<&str>>()[filepath.split(".").count() - 2];
    let extension = filepath.split(".").collect::<Vec<&str>>()[filepath.split(".").count() - 1];
    let filename = filepath.split("/").collect::<Vec<&str>>()[filepath.split("/").count() - 1]
        .split(".")
        .collect::<Vec<&str>>()[0];
    println!("Path: .{folder_path}/{filename}/0.{extension}");
    println!("Folder path: {folder_path}");
    println!("Extension: {extension}");
    println!("Filename: {filename}");
    match create_dir_all(format!(".{folder_path}")) {
        Ok(_) => {}
        Err(e) => {
            println!("Error when creating directory: {}", e);
            return;
        }
    };
    let mut i = 0;
    let parsed_split = parsed.chunks(split_size);
    for chunk in parsed_split {
        let mut file = match File::create(format!(".{folder_path}/{i}.{extension}")) {
            Ok(f) => f,
            Err(e) => {
                println!("Error when creating file: {}", e);
                return;
            }
        };
        let chunk = match serde_json::to_string(chunk) {
            Ok(c) => c,
            Err(e) => {
                println!("Error when parsing chunk: {}", e);
                return;
            }
        };
        match file.write_all(chunk.as_bytes()) {
            Ok(_) => {}
            Err(e) => {
                println!("Error when writing to file: {}", e);
                return;
            }
        };
        i += 1;
    }
}
