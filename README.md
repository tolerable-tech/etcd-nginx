Reads values from Etcd and generates nginx conf files.

Currently using confd.

NOTE
====

For some reason, when compiling with:

```
docker run --rm -it -v "$(pwd)":/home/rust/src ekidd/rust-musl-builder
```

it's still downloading a version of etcd that has serd compile errors, you have
to edit build.rs to look like:

```
#[cfg(not(feature = "serde_macros"))]
mod inner {
    extern crate serde_codegen;

    use std::env;
    use std::path::Path;

    const MODULES: &'static[&'static str] = &[
        "error",
        "keys",
        "stats",
        "version",
    ];

    pub fn main() {
        let out_dir = env::var_os("OUT_DIR").unwrap();

        for module in MODULES.iter() {
            let src = format!("src/{}_gen.rs", module);
            let src_path = Path::new(&src);
            let dst = format!("{}.rs", module);
            let dst_path = Path::new(&out_dir).join(&dst);

            serde_codegen::expand(&src_path, &dst_path).unwrap();
        }
    }
}

#[cfg(feature = "serde_macros")]
mod inner {
    pub fn main() {}
}

fn main() {
    inner::main();
}
```


```
Copyright © 2016 Tolerable Technology

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

Please see LICENSE.txt for a full copy of the GNU General Public License.
```
