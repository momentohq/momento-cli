use std::fs;

use momento::functions::WasmSource;

use crate::error::CliError;

pub fn read_wasm_file(wasm_file: String) -> Result<Vec<u8>, CliError> {
    let binary = fs::read(wasm_file).map_err(Into::<CliError>::into)?;
    if binary.is_empty() {
        return Err(CliError {
    msg: "Must provide a .wasm file compiled with wasm32-wasip2 to upload using the --wasm-file flag".to_string(),
  });
    }
    Ok(binary)
}

pub fn determine_wasm_source(
    wasm_file: Option<String>,
    id_uploaded_wasm: Option<String>,
    version_uploaded_wasm: Option<u32>,
) -> Result<WasmSource, CliError> {
    match (wasm_file, id_uploaded_wasm, version_uploaded_wasm) {
    (Some(wasm_file), None, None) => Ok(WasmSource::Inline(read_wasm_file(wasm_file)?)),
    (None, Some(id_uploaded_wasm), Some(version_uploaded_wasm)) => Ok(WasmSource::Reference {
      wasm_id: id_uploaded_wasm,
      version: version_uploaded_wasm,
    }),
    _ => Err(CliError {
      msg: "Must provide a .wasm file compiled with wasm32-wasip2 to upload using the --wasm-file flag or a previously uploaded wasm using the --id-uploaded-wasm and --version-uploaded-wasm flags".to_string(),
    }),
    }
}
