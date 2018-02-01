extern crate data_url;
extern crate rustc_test;
extern crate serde_json;

fn run_data_url(input: String, expected_mime: Option<String>, expected_body: Option<Vec<u8>>) {
    let url = data_url::DataUrl::process(&input);
    if let Some(expected_mime) = expected_mime {
        let url = url.unwrap();
        let (body, _) = url.decode_to_vec().unwrap();
        if expected_mime == "" {
            assert_eq!(*url.mime_type(), "text/plain;charset=US-ASCII")
        } else {
            assert_eq!(*url.mime_type(), &*expected_mime)
        }
        if let Some(expected_body) = expected_body {
            assert_eq!(body, expected_body)
        }
    } else if let Ok(url) = url {
        assert!(url.decode_to_vec().is_err(), "{:?}", url.mime_type())
    }
}

fn collect_data_url<F>(add_test: &mut F)
    where F: FnMut(String, bool, rustc_test::TestFn)
{
    let known_failures = [
        "data://test:test/,X",
        "data:;%62ase64,WA",
        "data:;base 64,WA",
        "data:;base64;,WA",
        "data:;base64;base64,WA",
        "data:;charset =x,X",
        "data:;charset,X",
        "data:;charset=,X",
        "data:text/plain;,X",
        "data:text/plain;a=\",\",X",
        "data:x/x;base64;base64,WA",
        "data:x/x;base64;base64x,WA",
        "data:x/x;base64;charset=x,WA",
        "data:x/x;base64;charset=x;base64,WA",
    ];

    let json = include_str!("data-urls.json");
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    for test in v.as_array().unwrap() {
        let input = test.get(0).unwrap().as_str().unwrap().to_owned();

        let expected_mime = test.get(1).unwrap();
        let expected_mime = if expected_mime.is_null() {
            None
        } else {
            Some(expected_mime.as_str().unwrap().to_owned())
        };

        let expected_body = test.get(2).map(json_byte_array);

        let should_panic = known_failures.contains(&&*input);
        add_test(
            format!("data: URL {:?}", input),
            should_panic,
            rustc_test::TestFn::dyn_test_fn(move || {
                run_data_url(input, expected_mime, expected_body)
            })
        );
    }
}

fn run_base64(input: String, expected: Option<Vec<u8>>) {
    let result = data_url::forgiving_base64::decode_to_vec(input.as_bytes());
    match (result, expected) {
        (Ok(bytes), Some(expected)) => assert_eq!(bytes, expected),
        (Ok(bytes), None) => panic!("Expected error, got {:?}", bytes),
        (Err(e), Some(expected)) => panic!("Expected {:?}, got error {:?}", expected, e),
        (Err(_), None) => {}
    }
}


fn collect_base64<F>(add_test: &mut F)
    where F: FnMut(String, bool, rustc_test::TestFn)
{
    let known_failures = [];

    let json = include_str!("base64.json");
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    for test in v.as_array().unwrap() {
        let input = test.get(0).unwrap().as_str().unwrap().to_owned();
        let expected = test.get(1).unwrap();
        let expected = if expected.is_null() {
            None
        } else {
            Some(json_byte_array(expected))
        };

        let should_panic = known_failures.contains(&&*input);
        add_test(
            format!("base64 {:?}", input),
            should_panic,
            rustc_test::TestFn::dyn_test_fn(move || {
                run_base64(input, expected)
            })
        );
    }
}

fn run_mime(input: String, expected: Option<String>) {
    let result = input.parse::<data_url::mime::Mime>();
    match (result, expected) {
        (Ok(bytes), Some(expected)) => assert_eq!(bytes, &*expected),
        (Ok(bytes), None) => panic!("Expected error, got {:?}", bytes),
        (Err(e), Some(expected)) => panic!("Expected {:?}, got error {:?}", expected, e),
        (Err(_), None) => {}
    }
}


fn collect_mime<F>(add_test: &mut F)
    where F: FnMut(String, bool, rustc_test::TestFn)
{
    // Many WPT tests fail with the mime crate’s parser,
    // since that parser is not written for the same spec.
    // Only run a few of them for now, since listing all the failures individually is not useful.
    let only_run_first_n_tests = 5;
    let known_failures = [
        "text/html;charset=gbk(",
    ];

    let json = include_str!("mime-types.json");
    let json2 = include_str!("generated-mime-types.json");
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    let v2: serde_json::Value = serde_json::from_str(json2).unwrap();
    let tests = v.as_array().unwrap().iter().chain(v2.as_array().unwrap());

    let mut last_comment = None;
    for test in tests.take(only_run_first_n_tests) {
        if let Some(s) = test.as_str() {
            last_comment = Some(s);
            continue
        }
        let input = test.get("input").unwrap().as_str().unwrap().to_owned();
        let expected = test.get("output").unwrap();
        let expected = if expected.is_null() {
            None
        } else {
            Some(expected.as_str().unwrap().to_owned())
        };

        let should_panic = known_failures.contains(&&*input);
        add_test(
            if let Some(s) = last_comment {
                format!("MIME type {:?} {:?}", s, input)
            } else {
                format!("MIME type {:?}", input)
            },
            should_panic,
            rustc_test::TestFn::dyn_test_fn(move || {
                run_mime(input, expected)
            })
        );
    }
}

fn json_byte_array(j: &serde_json::Value) -> Vec<u8> {
    j.as_array().unwrap().iter().map(|byte| {
        let byte = byte.as_u64().unwrap();
        assert!(byte <= 0xFF);
        byte as u8
    }).collect()
}

fn main() {
    let mut tests = Vec::new();
    {
        let mut add_one = |name: String, should_panic: bool, run: rustc_test::TestFn| {
            let mut desc = rustc_test::TestDesc::new(rustc_test::DynTestName(name));
            if should_panic {
                desc.should_panic = rustc_test::ShouldPanic::Yes
            }
            tests.push(rustc_test::TestDescAndFn { desc, testfn: run })
        };
        collect_data_url(&mut add_one);
        collect_base64(&mut add_one);
        collect_mime(&mut add_one);
    }
    rustc_test::test_main(&std::env::args().collect::<Vec<_>>(), tests)
}
