//! Browser worker regressions: the actual JS is checked against Rust SHA-256.
use super::{impressum, repo};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
const NONCE: &str = "AAAAAAAAAAAAAAAAAAAAAA";

// Node is only a test runner for browser JavaScript, never a tool dependency.
fn node(script: &str, args: &[String]) -> Option<Value> {
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("browser JavaScript parity test skipped: Node unavailable");
        return None;
    }
    let mut child = Command::new("node")
        .current_dir(repo())
        .args(["-e", script])
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if start.elapsed() >= Duration::from_secs(60) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("browser parity test timed out");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "node parity failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Some(serde_json::from_slice(&output.stdout).unwrap())
}
fn worker(script: &str) -> Option<Value> {
    let script = format!("const w=require(process.argv[1]);\n{script}");
    node(&script, &["./web/site/impressum-worker.js".into()])
}
fn solves(part: usize, counter: u64, bits: u32) -> bool {
    let digest = Sha256::digest(format!("{NONCE}:{part}:{counter}"));
    let zeros = digest.iter().take_while(|b| **b == 0).count() * 8;
    let zeros = zeros
        + digest
            .iter()
            .find(|b| **b != 0)
            .map_or(0, |b| b.leading_zeros() as usize);
    zeros >= bits as usize
}
#[test]
fn test_the_hand_written_sha256_agrees_with_hashlib() {
    let samples = vec![
        String::new(),
        "abc".into(),
        "a".repeat(55),
        "a".repeat(56),
        "b".repeat(63),
        "c".repeat(64),
        "d".repeat(119),
        "e".repeat(120),
        "f".repeat(130),
        "g".repeat(200),
        "AAAAAAAAAAAAAAAAAAAAAA:3:1234567".into(),
        "The quick brown fox jumps over the lazy dog".into(),
        "-_09azAZ:~!#$%&()*+,./;<=>?@[]^`{|}".into(),
    ];
    let Some(got) = worker(&format!(
        "console.log(JSON.stringify({}.map(w.hex)));",
        serde_json::to_string(&samples).unwrap()
    )) else {
        return;
    };
    assert_eq!(got.as_array().unwrap().len(), samples.len());
    for (text, digest) in samples.iter().zip(got.as_array().unwrap()) {
        assert_eq!(
            digest.as_str().unwrap(),
            impressum::hex(&Sha256::digest(text.as_bytes()))
        );
    }
}
#[test]
fn test_the_known_digests_are_right_in_both() {
    let Some(got) = worker("console.log(JSON.stringify(['','abc'].map(w.hex)));") else {
        return;
    };
    assert_eq!(
        got,
        json!([
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        ])
    );
}
#[test]
fn test_the_solver_finds_the_smallest_counter_of_every_part() {
    for bits in [1, 7, 8, 9, 10, 12] {
        let Some(got) = worker(&format!(
            "console.log(JSON.stringify(w.solve('{NONCE}',3,{bits})));"
        )) else {
            return;
        };
        let counters = got.as_array().unwrap();
        assert_eq!(counters.len(), 3);
        for (part, counter) in counters.iter().enumerate() {
            let counter = counter.as_u64().unwrap();
            assert!(solves(part, counter, bits));
            for below in 0..counter {
                assert!(!solves(part, below, bits));
            }
        }
    }
}
#[test]
fn test_the_solver_agrees_with_the_rust_service_vector() {
    for (bits, expected) in [(8, json!([218, 8, 247])), (9, json!([961, 1478, 247]))] {
        let Some(got) = worker(&format!(
            "console.log(JSON.stringify(w.solve('{NONCE}',3,{bits})));"
        )) else {
            return;
        };
        assert_eq!(got, expected);
    }
}
#[test]
fn test_the_solver_reports_progress_after_every_part() {
    let Some(got) = worker(&format!(
        "const seen=[]; const c=w.solve('{NONCE}',3,4,(done,all)=>seen.push([done,all])); console.log(JSON.stringify({{seen,c}}));"
    )) else {
        return;
    };
    assert_eq!(got, json!({"seen":[[1,3],[2,3],[3,3]],"c":[4,8,2]}));
}
#[test]
fn test_a_nonce_the_worker_cannot_take_throws() {
    let Some(got) = worker(
        "let error='';try { w.hex('x'.repeat(201)); } catch(e) { error=e.message; } console.log(JSON.stringify(error));",
    ) else {
        return;
    };
    assert!(got.as_str().unwrap().contains("too long"));
}
#[test]
fn rust_blob_roundtrips_through_actual_browser_decoder() {
    let fields = json!({"name":"Jürgen Müller","street":"Hauptstraße 5","postcode":"80331","city":"Köln-Süd","country_de":"Österreich","country_en":"Austria","email":"j.mueller@example.net","phone":"+43 (0) 660 987 6543"});
    let blob = impressum::encode(&fields, None).unwrap();
    let script = r"
const fs=require('node:fs');
const crypto=require('node:crypto').webcrypto;
const source=fs.readFileSync(process.argv[1],'utf8');
const start=source.indexOf('  function hexBytes(');
const end=source.indexOf('  // ------------------------------------------------------------ the page',start);
if(start<0||end<0)throw new Error('browser decoder boundary missing');
const constants=['MAGIC','VERSION','NONCE_LEN','SCRIPT_KEY'].map(k=>{
  const match=source.match(new RegExp('var '+k+' = [^;]+;'));
  if(!match)throw new Error('browser constant missing'); return match[0];
}).join('\n');
const decode=new Function('crypto',constants+'\n'+source.slice(start,end)+'\nreturn decode;')(crypto);
decode(process.argv[2]).then(fields=>{
 const out={}; for(const [key,pairs] of Object.entries(fields)) {
   out[key]=pairs.sort((a,b)=>a[0]-b[0]).map(p=>String.fromCodePoint(p[1])).join('');
 } console.log(JSON.stringify(out));
}).catch(()=>{process.exitCode=1;});
";
    let Some(got) = node(
        script,
        &[
            repo()
                .join("web/site/impressum.js")
                .to_string_lossy()
                .into_owned(),
            blob,
        ],
    ) else {
        return;
    };
    assert_eq!(got, fields);
}
#[test]
fn feedback_envelope_constants_agree_with_native_encoder() {
    let mut source = String::new();
    std::fs::File::open(repo().join("crates/baylee-feedback/src/impressum.rs"))
        .unwrap()
        .read_to_string(&mut source)
        .unwrap();
    for (pattern, expected) in [
        (
            r"pub const VERSION: u8 = (\d+);",
            impressum::VERSION.to_string(),
        ),
        (
            r"pub const NONCE_LEN: usize = (\d+);",
            impressum::NONCE_LEN.to_string(),
        ),
    ] {
        let re = regex::Regex::new(pattern).unwrap();
        assert_eq!(&re.captures(&source).unwrap()[1], expected);
    }
    assert!(source.contains("pub const MAGIC: &[u8; 4] = b\"BYIM\";"));
}
