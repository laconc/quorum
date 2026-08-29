//! Fingerprinted static assets.
//!
//! Assets are embedded in the binary and served under a URL containing a hash
//! of their contents. That makes them safe to cache forever: when the content
//! changes the URL changes, so a client never has to be told to revalidate and
//! never serves a stale script against fresh HTML.
//!
//! Embedding rather than reading from disk means the binary is the whole
//! deployment artifact — there is no way to run a build whose assets and code
//! disagree.

use std::collections::HashMap;

/// One embedded asset.
#[derive(Debug, Clone, Copy)]
pub struct Asset {
    /// The logical name, as written in a template.
    pub name: &'static str,
    /// The bytes served.
    pub body: &'static [u8],
    /// The `Content-Type` to serve it under.
    pub content_type: &'static str,
}

/// Everything served under `/static/`.
const ASSETS: &[Asset] = &[
    Asset {
        name: "htmx.min.js",
        body: include_bytes!("../static/htmx.min.js"),
        // Vendored rather than loaded from a content delivery network: a
        // third-party script origin would have to be admitted by the Content
        // Security Policy, which is a supply-chain hole, and it would make the
        // application fail in exactly the conditions where it matters most.
        content_type: "application/javascript; charset=utf-8",
    },
    Asset {
        name: "app.css",
        body: include_bytes!("../static/app.css"),
        content_type: "text/css; charset=utf-8",
    },
];

/// Resolved assets, addressed by their fingerprinted path.
#[derive(Debug, Clone)]
pub struct Assets {
    by_path: HashMap<String, Asset>,
    urls: HashMap<&'static str, String>,
}

impl Assets {
    /// Fingerprint every embedded asset.
    ///
    /// # Panics
    ///
    /// Panics if two assets share a logical name, which is a mistake in this
    /// module's embedded asset table rather than a runtime condition.
    #[must_use]
    pub fn load() -> Self {
        let mut by_path = HashMap::new();
        let mut urls = HashMap::new();

        for asset in ASSETS {
            let digest = app_crypto::hash(asset.body);
            // Sixteen hex characters is 64 bits of the digest — far more than
            // enough to distinguish a handful of files, and short enough to
            // keep the URL readable in a screenshot.
            let fingerprint: String = digest.to_hex().chars().take(16).collect();
            let path = format!("/static/{fingerprint}/{}", asset.name);

            let previous = urls.insert(asset.name, path.clone());
            assert!(previous.is_none(), "duplicate asset name: {}", asset.name);
            by_path.insert(path, *asset);
        }

        Self { by_path, urls }
    }

    /// The fingerprinted URL for a logical asset name.
    ///
    /// # Panics
    ///
    /// Panics if the name is not an embedded asset. Templates reference assets
    /// by literal name, so a miss is a build-time mistake surfaced at startup
    /// rather than a broken page in production.
    #[must_use]
    pub fn url(&self, name: &str) -> &str {
        self.urls
            .get(name)
            .unwrap_or_else(|| panic!("no embedded asset named {name}"))
    }

    /// Look up an asset by its fingerprinted path.
    #[must_use]
    pub fn get(&self, path: &str) -> Option<&Asset> {
        self.by_path.get(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_asset_is_addressable_by_its_fingerprinted_url() {
        let assets = Assets::load();
        for asset in ASSETS {
            let url = assets.url(asset.name);
            let found = assets.get(url).expect("asset resolves by its own url");
            assert_eq!(found.body, asset.body);
        }
    }

    #[test]
    fn fingerprints_are_stable_across_loads() {
        // If these drifted, every screenshot would churn on every run.
        let a = Assets::load();
        let b = Assets::load();
        for asset in ASSETS {
            assert_eq!(a.url(asset.name), b.url(asset.name));
        }
    }

    #[test]
    fn fingerprints_differ_between_assets() {
        let assets = Assets::load();
        assert_ne!(assets.url("htmx.min.js"), assets.url("app.css"));
    }
}
