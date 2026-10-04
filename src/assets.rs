//! The library's icon set, compiled in, plus the chaining that lets a host bring its own.
//!
//! GPUI resolves `svg()` paths through the single `AssetSource` an app registers, and
//! `Application::with_assets` takes exactly one (`gpui-ce/src/app.rs:202`). A library therefore
//! cannot simply ship icons: it has to hand the app something that *composes* with what the app was
//! going to register anyway.
//!
//! ```ignore
//! application().with_assets(nexus_look::Assets.chain(MyIcons)).run(..)
//! ```
//!
//! Window controls, `check`/`close`/`alert`/`chevron`/`search`/`folder`/`plus`/`gear` are part of
//! the language — they have to come from one place or every project ends up with its own
//! slightly-different gutter. Icons that name *your* product (`download`, `pause`, a logo) belong
//! to you.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

pub use crate::icons::*;

macro_rules! icons {
    ($($file:literal),* $(,)?) => {
        /// Every icon the library serves, as `(path, bytes)`. The served path carries a
        /// `nexus-look/` prefix so a host's icons cannot collide with ours by accident.
        pub const ICONS: &[(&str, &[u8])] = &[
            $({
                let path: &str = concat!("nexus-look/", $file);
                let bytes: &[u8] = include_bytes!(concat!("../assets/icons/", $file));
                (path, bytes)
            },)*
        ];
    };
}

icons![
    "alert.svg",
    "check.svg",
    "chevron-down.svg",
    "chevron-right.svg",
    "close.svg",
    "folder.svg",
    "gear.svg",
    "link.svg",
    "max.svg",
    "min.svg",
    "minus.svg",
    "plus.svg",
    "restore.svg",
    "search.svg",
    "trash.svg",
];

fn lookup(path: &str) -> Option<Cow<'static, [u8]>> {
    ICONS
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, bytes)| Cow::Borrowed(*bytes))
}

/// The library's own icons.
pub struct Assets;

impl Assets {
    /// Serve the library's icons first, then `other`'s. This is the only supported way to register
    /// the library with an app that has icons of its own.
    pub fn chain<A: AssetSource>(self, other: A) -> Chained<A> {
        Chained { other }
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(lookup(path))
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .map(|(name, _)| SharedString::from(*name))
            .collect())
    }
}

/// Two asset sources as one, the library's icons taking precedence — a host cannot shadow a
/// window-control icon by accident, and does not have to know which names the library claimed.
pub struct Chained<A> {
    other: A,
}

impl<A: AssetSource> AssetSource for Chained<A> {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match lookup(path) {
            Some(bytes) => Ok(Some(bytes)),
            None => self.other.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut names = Assets.list(path)?;
        names.extend(self.other.list(path)?);
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Host;

    impl AssetSource for Host {
        fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
            Ok((path == "host/logo.svg").then(|| Cow::Borrowed(&b"<svg/>"[..])))
        }

        fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
            Ok(vec![SharedString::from("host/logo.svg")])
        }
    }

    #[test]
    fn the_chain_serves_both_sets() {
        let chained = Assets.chain(Host);
        assert!(chained.load(CLOSE).unwrap().is_some());
        assert!(chained.load("host/logo.svg").unwrap().is_some());
        assert!(chained.load("nobody/knows.svg").unwrap().is_none());
    }

    #[test]
    fn the_library_wins_a_name_it_owns() {
        // A host registering `nexus-look/close.svg` itself must not be able to change the window
        // controls out from under the library.
        struct Impostor;
        impl AssetSource for Impostor {
            fn load(&self, _path: &str) -> Result<Option<Cow<'static, [u8]>>> {
                Ok(Some(Cow::Borrowed(b"not an icon")))
            }
            fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
                Ok(Vec::new())
            }
        }
        let chained = Assets.chain(Impostor);
        let served = chained.load(CLOSE).unwrap().unwrap();
        assert_ne!(served.as_ref(), b"not an icon");
    }

    #[test]
    fn every_named_icon_is_actually_embedded() {
        for path in [
            ALERT,
            CHECK,
            CHEVRON_DOWN,
            CHEVRON_RIGHT,
            CLOSE,
            FOLDER,
            GEAR,
            LINK,
            MAXIMIZE,
            MINIMIZE,
            MINUS,
            PLUS,
            RESTORE,
            SEARCH,
            TRASH,
        ] {
            assert!(lookup(path).is_some(), "{path} is missing from the bundle");
        }
        // And every embedded icon is actually addressable — a shipped file nobody can reach is
        // bytes in the binary for nothing.
        for (name, _) in ICONS {
            assert!(lookup(name).is_some(), "{name} is embedded but unlisted");
        }
    }
}
