//! Build-time evaluated expressions
//!
//! `cconst` allows defining constants at build time of any type that
//! implements the `Copy` trait. Values are generated through `build.rs`:
//!
//! ```
//! // build.rs
//! #[macro_use]
//! extern crate cconst;
//!
//! use std::net::Ipv4Addr;
//!
//! let mut cs = CopyConsts::new();
//! cs.add_const("default_ns", "::std::net::Ipv4Addr", {
//!     Ipv4Addr::new(8, 8, 8, 8)
//! });
//! cs.write_code().unwrap();
//!
//! ```
//!
//! Once set up, they can be included using the `cconst!` macro:
//!
//! ```ignore
//! // main.rs
//! #[macro_use]
//! extern crate cconst;
//!
//! include!(cconst!(default_ns));
//!
//! fn main() {
//!     println!("default_ns: {:?}", default_ns());
//! }
//! ```
//!
//! # Internals
//!
//! `cconst` works by serializing the value defined in `build.rs` into
//! byte-slice literals and including those where applicable. The example above
//! results in roughly the following generated code:
//!
//! ```ignore
//! #[inline]
//! fn default_ns() -> &'static ::std::net::Ipv4Addr {
//!     const BUF: &[u8] = &[0x08, 0x08, 0x08, 0x08, ];
//!     unsafe { &*(BUF.as_ptr() as *const ::std::net::Ipv4Addr) }
//! }
//! ```
//!
//! Calling `default_ns()` should result in an inlined pointer cast and little,
//! if any overhead.
//!
//! ## TODO
//!
//! [ ] `#[no_std]` support

/// Imports a stored constant
#[macro_export]
macro_rules! cconst {
    ($fname:ident) => (concat!(env!("OUT_DIR"), "/cconst-", stringify!($fname), ".rs"))
}

/// Creates a constant for inclusion using `cconst!`.
///
/// This macro should be preferred over `CopyConsts::add_const`, as it provides
/// additional type safety.
#[macro_export]
macro_rules! add_const {
    ($cconsts:expr, $fname: expr, $ctype:ty, $val:expr) => (
        let mat: $ctype = $val;
        $cconsts.add_const($fname, stringify!($ctype), &mat);
        )
}

use std::{collections, env, fs, io};
use std::io::Write;
use std::mem::size_of;

fn marshall_value<T: Copy>(val: &T) -> String {
    let vptr = val as *const _ as *const u8;

    let mut rexpr = String::new();
    rexpr += "&[";

    for i in 0..size_of::<T>() {
        rexpr.push_str(&format!("0x{:02X}, ", unsafe { *vptr.offset(i as isize) }));
    }

    rexpr += "]";

    rexpr
}

fn create_constant_func<T: Copy>(fname: &str, typename: &str, val: &T) -> String {
    let sval = marshall_value(val);

    format!("#[inline]\nfn {}() -> &'static {} {{
    const BUF: &[u8] = {};
    unsafe {{ &*(BUF.as_ptr() as *const {}) }}
}}\n",
            fname,
            typename,
            sval,
            typename)
}

/// Manage `build.rs` constructed constants
pub struct CopyConsts(collections::HashMap<String, String>);


fn build_output_path(fname: &str) -> Result<String, env::VarError> {
    Ok(env::var("OUT_DIR")? + "/cconst-" + fname + ".rs")
}

impl CopyConsts {
    /// Create new set of compile time functions
    pub fn new() -> CopyConsts {
        CopyConsts(collections::HashMap::new())
    }

    /// Add constant
    ///
    /// Adds a value to be stored as a compile time constant, with an internal
    /// name of `fname`.
    ///
    /// `typename` is required to output generated code, but not checked. For
    /// this reason using the `add_const!` macro instead of this function
    /// should be preferred.
    pub fn add_const<T: Copy>(&mut self, fname: &str, typename: &str, val: &T) {
        self.0
            .insert(fname.to_owned(), create_constant_func(fname, typename, val));
    }

    /// Write out code for compile-time constant generation.
    pub fn write_code(&self) -> io::Result<()> {
        for (fname, buf) in &self.0 {
            let output_path =
                build_output_path(fname)
                    .map_err(|_| io::Error::new(io::ErrorKind::Other, "missing OUT_PATH"))?;

            write!(io::stdout(), "OUTPUT PATH {:?}", output_path).unwrap();
            let mut fp = fs::File::create(output_path)?;
            fp.write_all(buf.as_bytes())?;
        }

        Ok(())
    }
}
