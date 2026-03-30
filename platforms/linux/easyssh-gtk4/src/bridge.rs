use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;
use serde::{Deserialize, Serialize};

pub struct BridgeHandle {
    ptr: *mut c_void,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub group_id: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerGroup {
    pub id: String,
    pub name: String,
}

extern "C" {
    fn easyssh_init() -> *mut c_void;
    fn easyssh_destroy(handle: *mut c_void);
    fn easyssh_get_servers(handle: *mut c_void) -> *mut c_char;
    fn easyssh_get_groups(handle: *mut c_void) -> *mut c_char;
    fn easyssh_add_server(handle: *mut c_void, json: *const c_char) -> c_int;
    fn easyssh_delete_server(handle: *mut c_void, id: *const c_char) -> c_int;
    fn easyssh_connect_native(handle: *mut c_void, id: *const c_char) -> c_int;
    fn easyssh_free_string(s: *mut c_char);
}

impl BridgeHandle {
    pub fn new() -> Result<Self, String> {
        let ptr = unsafe { easyssh_init() };
        if ptr.is_null() {
            return Err("Failed to initialize".to_string());
        }
        Ok(Self { ptr })
    }

    pub fn get_servers(&self) -> Result<Vec<Server>, String> {
        unsafe {
            let ptr = easyssh_get_servers(self.ptr);
            if ptr.is_null() {
                return Ok(Vec::new());
            }

            let c_str = CStr::from_ptr(ptr).to_str()
                .map_err(|e| format!("Invalid UTF-8: {}", e))?;

            let result = serde_json::from_str(c_str)
                .map_err(|e| format!("JSON parse error: {}", e));

            easyssh_free_string(ptr);
            result
        }
    }

    pub fn get_groups(&self) -> Result<Vec<ServerGroup>, String> {
        unsafe {
            let ptr = easyssh_get_groups(self.ptr);
            if ptr.is_null() {
                return Ok(Vec::new());
            }

            let c_str = CStr::from_ptr(ptr).to_str()
                .map_err(|e| format!("Invalid UTF-8: {}", e))?;

            let result = serde_json::from_str(c_str)
                .map_err(|e| format!("JSON parse error: {}", e));

            easyssh_free_string(ptr);
            result
        }
    }

    pub fn add_server(&self, server: &Server) -> Result<(), String> {
        unsafe {
            let json = serde_json::to_string(server)
                .map_err(|e| format!("Serialize error: {}", e))?;
            let c_str = CString::new(json)
                .map_err(|e| format!("CString error: {}", e))?;

            let result = easyssh_add_server(self.ptr, c_str.as_ptr());
            if result == 0 { Ok(()) } else { Err("Failed to add server".to_string()) }
        }
    }

    pub fn delete_server(&self, id: &str) -> Result<(), String> {
        unsafe {
            let c_id = CString::new(id)
                .map_err(|e| format!("CString error: {}", e))?;
            let result = easyssh_delete_server(self.ptr, c_id.as_ptr());
            if result == 0 { Ok(()) } else { Err("Failed to delete".to_string()) }
        }
    }

    pub fn connect_native(&self, id: &str) -> Result<(), String> {
        unsafe {
            let c_id = CString::new(id)
                .map_err(|e| format!("CString error: {}", e))?;
            let result = easyssh_connect_native(self.ptr, c_id.as_ptr());
            if result == 0 { Ok(()) } else { Err("Connection failed".to_string()) }
        }
    }
}

impl Drop for BridgeHandle {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { easyssh_destroy(self.ptr) };
        }
    }
}
