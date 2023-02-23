use std::{env, io::BufWriter, path::PathBuf, fs::File};

use gl_generator::{Registry, DebugStructGenerator};

fn main() {
    let dest = env::var("OUT_DIR").unwrap();
    let mut file = BufWriter::new(File::create(PathBuf::from(dest).join("bindings.rs")).unwrap());

    Registry::new(gl_generator::Api::Gl, (3, 3), gl_generator::Profile::Core, gl_generator::Fallbacks::All, ["GL_EXT_debug_label"])
        .write_bindings(DebugStructGenerator, &mut file)
        .unwrap();
}