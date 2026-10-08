// src/environments/environment_handle.rs

/// convenient for future uses when environment manager will also provision vms etc.

//use crate::models::EnvironmentKind;


#[derive(Debug, Clone)]
pub struct EnvironmentHandle {
    //pub kind: EnvironmentKind,
    
    pub id  : String,
    pub name: String,
    pub ip  : String,

    //snapshot_id  : "clean",
    //vm_id        : ...,
    //created_at   : ...,
}