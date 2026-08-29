//! Test the testers.
//!
//! A harness that has never been shown to catch anything is not a harness. Each
//! test here plants a deliberate defect and asserts that the check meant to
//! catch it does. A build where a planted defect goes uncaught is red, and that
//! is the point: it is the only way to know the real checks are load-bearing
//! rather than decorative.

use app_crypto::{canonicalize, hash_chain_link};
use serde_json::{Map, Value, json};

/// A canonical encoder that sorts object keys by UTF-8 bytes instead of UTF-16
/// code units.
///
/// This is the subtle version of the defect, and the one a real implementation
/// is most likely to have: it agrees with the correct encoder on every ASCII
/// input, so an ordinary test suite passes. It diverges only above the basic
/// multilingual plane — which is to say, on someone's name.
fn encode_sorting_by_utf8(value: &Value) -> Vec<u8> {
    fn go(value: &Value, out: &mut String) {
        match value {
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort(); // UTF-8 byte order — the defect.
                out.push('{');
                for (i, key) in keys.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push_str(&serde_json::to_string(key).expect("string"));
                    out.push(':');
                    go(&map[key.as_str()], out);
                }
                out.push('}');
            }
            Value::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    go(item, out);
                }
                out.push(']');
            }
            other => out.push_str(&serde_json::to_string(other).expect("scalar")),
        }
    }
    let mut out = String::new();
    go(value, &mut out);
    out.into_bytes()
}

/// The input that separates UTF-16 ordering from UTF-8 ordering.
fn astral_and_fullwidth() -> (Value, Value) {
    let fullwidth = '\u{ff3a}'.to_string();
    let astral = '\u{10000}'.to_string();

    let mut forward = Map::new();
    forward.insert(fullwidth.clone(), json!(1));
    forward.insert(astral.clone(), json!(2));

    let mut reversed = Map::new();
    reversed.insert(astral, json!(2));
    reversed.insert(fullwidth, json!(1));

    (Value::Object(forward), Value::Object(reversed))
}

#[test]
fn the_ordering_check_catches_an_encoder_that_sorts_by_utf8() {
    let (forward, reversed) = astral_and_fullwidth();

    // Both orderings of the same object must encode identically. The planted
    // encoder still satisfies that, because it sorts consistently — just
    // wrongly. What it gets wrong is *which* order, so compare against the
    // real encoder to expose it.
    let correct = canonicalize(&forward).expect("float-free");
    let planted = encode_sorting_by_utf8(&forward);

    assert_ne!(
        correct, planted,
        "the planted UTF-8-sorting encoder went undetected; the ordering rule \
         is not actually being checked, and any record containing an astral \
         character would verify differently on a different implementation"
    );

    // And the real encoder is order-independent, which the planted one must
    // also be for the comparison above to be the meaningful difference.
    assert_eq!(
        canonicalize(&forward).expect("float-free"),
        canonicalize(&reversed).expect("float-free"),
    );
}

/// An encoder that emits object keys in insertion order.
///
/// The blunt version of the defect. It fails on any two-key object, which is
/// why it exists: if this were ever to pass, the ordering property is not being
/// evaluated at all.
fn encode_in_insertion_order(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).expect("serialises")
}

#[test]
fn the_ordering_check_catches_an_encoder_that_does_not_sort() {
    let forward = json!({"b": 1, "a": 2});
    let reversed = json!({"a": 2, "b": 1});

    assert_ne!(
        encode_in_insertion_order(&forward),
        encode_in_insertion_order(&reversed),
        "the planted unsorted encoder produced order-independent output, which \
         means this fixture is no longer exercising the property"
    );

    assert_eq!(
        canonicalize(&forward).expect("float-free"),
        canonicalize(&reversed).expect("float-free"),
        "the real encoder must be order-independent"
    );
}

#[test]
fn the_chain_check_catches_a_link_that_ignores_its_predecessor() {
    // A chain whose links do not depend on what came before is a list of
    // hashes: any entry could be replaced without disturbing anything after it.
    fn planted_link(_prev: Option<&app_crypto::Digest>, entry: &[u8]) -> app_crypto::Digest {
        app_crypto::hash(entry)
    }

    let a = hash_chain_link(None, b"first");
    let b = hash_chain_link(None, b"second");

    assert_eq!(
        planted_link(Some(&a), b"same").to_hex(),
        planted_link(Some(&b), b"same").to_hex(),
        "the planted link is supposed to ignore its predecessor"
    );
    assert_ne!(
        hash_chain_link(Some(&a), b"same").to_hex(),
        hash_chain_link(Some(&b), b"same").to_hex(),
        "the real chain link must depend on its predecessor, or tampering with \
         an earlier entry would leave every later link intact"
    );
}
