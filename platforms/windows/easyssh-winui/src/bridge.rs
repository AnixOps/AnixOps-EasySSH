use std::ffi::{c_char, c_void, CStr, CString};
use serde::{Deserialize, Serialize};

pub struct BridgeHandle {
    ptr: *mut c_void,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerViewModel {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupViewModel {
    pub id: String,
    pub name: String,
}

extern "C" {
    fn easyssh_init() -> *mut c_void;
    fn easyssh_destroy(handle: *mut c_void);
    fn easyssh_get_servers(handle: *mut c_void) -> *mut c_char;
    fn easyssh_free_string(s: *mut c_char);
    fn easyssh_connect_native(handle: *mut c_void, id: *const c_char) -> i32;
}

impl BridgeHandle {
    pub fn new() -> anyhow::Result<Self> {
        let ptr = unsafe { easyssh_init() };
        if ptr.is_null() {
            anyhow::bail!("Failed to initialize EasySSH core");
        }
        Ok(Self { ptr })
    }

    pub fn get_servers(&self) -> anyhow::Result<Vec<ServerViewModel>> {
        unsafe {
            let ptr = easyssh_get_servers(self.ptr);
            if ptr.is_null() {
                return Ok(Vec::new());
            }

            let result = (|| -> anyhow::Result<_> {
                let c_str = CStr::from_ptr(ptr).to_str()?;
                let servers: Vec<ServerViewModel> = serde_json::from_str(c_str)?;
                Ok(servers)
            })();

            easyssh_free_string(ptr);
            result
        }
    }

    pub fn connect_native(&self, id: &str) -> anyhow::Result<()> {
        unsafe {
            let c_id = CString::new(id)?;
            let result = easyssh_connect_native(self.ptr, c_id.as_ptr());
            if result == 0 { Ok(()) } else { anyhow::bail!("Connection failed") }
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
