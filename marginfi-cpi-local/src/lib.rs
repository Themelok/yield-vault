// DO NOT add zero_copy/packed for foreign program accounts.
// Many aren’t POD and it will fail to compile.

anchor_gen::generate_cpi_crate!("src/idl.json");