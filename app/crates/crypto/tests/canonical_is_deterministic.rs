//! The property the audit hash chain rests on.
//!
//! Section 5.8 of the design document is explicit that a non-deterministic
//! canonical encoding makes the chain decorative. These are the tests that make
//! "deterministic" a checked claim rather than an intention.

use app_crypto::{canonicalize, hash_chain_link};
use proptest::prelude::*;
use serde_json::{Map, Value};

/// Generate arbitrary float-free JSON, which is the domain this encoder is
/// defined over.
fn json_value() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(|i| Value::Number(i.into())),
        ".{0,24}".prop_map(Value::String),
    ];
    leaf.prop_recursive(4, 48, 6, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..6).prop_map(Value::Array),
            prop::collection::vec((".{0,12}", inner), 0..6).prop_map(|pairs| {
                Value::Object(pairs.into_iter().collect::<Map<String, Value>>())
            }),
        ]
    })
}

/// Rebuild a value with every object's keys inserted in reverse order. A
/// canonical encoder must not be able to tell the difference.
fn reverse_key_order(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(reverse_key_order).collect()),
        Value::Object(map) => {
            let mut rebuilt = Map::new();
            for (k, v) in map.iter().collect::<Vec<_>>().into_iter().rev() {
                rebuilt.insert(k.clone(), reverse_key_order(v));
            }
            Value::Object(rebuilt)
        }
        other => other.clone(),
    }
}

proptest! {
    #[test]
    fn encoding_is_stable_across_repeated_calls(v in json_value()) {
        let first = canonicalize(&v).expect("float-free by construction");
        let second = canonicalize(&v).expect("float-free by construction");
        prop_assert_eq!(first, second);
    }

    #[test]
    fn encoding_ignores_object_key_insertion_order(v in json_value()) {
        let direct = canonicalize(&v).expect("float-free by construction");
        let reordered = canonicalize(&reverse_key_order(&v)).expect("float-free by construction");
        prop_assert_eq!(direct, reordered);
    }

    #[test]
    fn encoding_survives_a_json_round_trip(v in json_value()) {
        let text = serde_json::to_string(&v).expect("serialises");
        let parsed: Value = serde_json::from_str(&text).expect("parses");
        prop_assert_eq!(
            canonicalize(&v).expect("float-free"),
            canonicalize(&parsed).expect("float-free")
        );
    }

    #[test]
    fn encoding_is_valid_utf8(v in json_value()) {
        let bytes = canonicalize(&v).expect("float-free by construction");
        prop_assert!(std::str::from_utf8(&bytes).is_ok());
    }

    #[test]
    fn distinct_values_encode_distinctly(a in json_value(), b in json_value()) {
        // Injectivity is what makes the chain meaningful: if two different
        // entries could encode identically, one could be substituted for the
        // other without breaking a single link.
        let ea = canonicalize(&a).expect("float-free");
        let eb = canonicalize(&b).expect("float-free");
        prop_assert_eq!(a == b, ea == eb);
    }

    #[test]
    fn a_chain_detects_any_alteration(
        entries in prop::collection::vec(json_value(), 1..8),
        target in 0usize..8,
    ) {
        prop_assume!(target < entries.len());

        let chain = |entries: &[Value]| {
            let mut prev = None;
            for entry in entries {
                let encoded = canonicalize(entry).expect("float-free");
                prev = Some(hash_chain_link(prev.as_ref(), &encoded));
            }
            prev.expect("at least one entry")
        };

        let original = chain(&entries);

        let mut altered = entries.clone();
        altered[target] = Value::String("tampered".to_owned());
        prop_assume!(altered[target] != entries[target]);

        prop_assert_ne!(
            original.to_hex(),
            chain(&altered).to_hex(),
            "altering entry {} left the chain head unchanged", target
        );
    }
}
