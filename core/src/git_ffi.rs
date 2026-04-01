use crate::git_client::GitClient;
use crate::git_types::*;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// FFI-safe Git client wrapper
pub struct FfiGitClient {
    client: Arc<Mutex<GitClient>>,
}

/// Opaque handle for FFI
pub type GitClientHandle = *mut FfiGitClient;

/// Initialize the FFI git module
#[no_mangle]
pub extern "C" fn git_client_new() -> GitClientHandle {
    let client = FfiGitClient {
        client: Arc::new(Mutex::new(GitClient::new())),
    };
    Box::into_raw(Box::new(client))
}

/// Free the git client
#[no_mangle]
pub extern "C" fn git_client_free(handle: GitClientHandle) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle);
        }
    }
}

/// Open a repository
#[no_mangle]
pub extern "C" fn git_client_open(handle: GitClientHandle, path: *const c_char) -> c_int {
    if handle.is_null() || path.is_null() {
        return -1;
    }

    let path = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(s) => PathBuf::from(s),
            Err(_) => return -1,
        }
    };

    let client = unsafe { &*handle };
    let mut client_guard = client.client.lock().unwrap();

    match client_guard.open(&path) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Initialize a repository
#[no_mangle]
pub extern "C" fn git_client_init(
    handle: GitClientHandle,
    path: *const c_char,
    bare: c_int,
) -> c_int {
    if handle.is_null() || path.is_null() {
        return -1;
    }

    let path = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(s) => PathBuf::from(s),
            Err(_) => return -1,
        }
    };

    let client = unsafe { &*handle };
    let mut client_guard = client.client.lock().unwrap();

    match client_guard.init(&path, bare != 0) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Clone a repository
#[no_mangle]
pub extern "C" fn git_client_clone(
    handle: GitClientHandle,
    url: *const c_char,
    path: *const c_char,
) -> c_int {
    if handle.is_null() || url.is_null() || path.is_null() {
        return -1;
    }

    let url = unsafe {
        match CStr::from_ptr(url).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return -1,
        }
    };

    let path = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(s) => PathBuf::from(s),
            Err(_) => return -1,
        }
    };

    let client = unsafe { &*handle };
    let mut client_guard = client.client.lock().unwrap();

    let options = CloneOptions::default();

    match client_guard.clone(&url, &path, &options) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Commit changes
#[no_mangle]
pub extern "C" fn git_client_commit(
    handle: GitClientHandle,
    message: *const c_char,
    amend: c_int,
    out_commit_id: *mut *mut c_char,
) -> c_int {
    if handle.is_null() || message.is_null() || out_commit_id.is_null() {
        return -1;
    }

    let message = unsafe {
        match CStr::from_ptr(message).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return -1,
        }
    };

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    match client_guard.commit(&message, amend != 0) {
        Ok(commit_id) => {
            let c_string = match CString::new(commit_id) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            unsafe {
                *out_commit_id = c_string.into_raw();
            }
            0
        }
        Err(_) => -1,
    }
}

/// Stage files
#[no_mangle]
pub extern "C" fn git_client_stage(
    handle: GitClientHandle,
    paths: *const *const c_char,
    count: c_int,
) -> c_int {
    if handle.is_null() || paths.is_null() || count <= 0 {
        return -1;
    }

    let paths: Vec<String> = unsafe {
        let slice = std::slice::from_raw_parts(paths, count as usize);
        slice
            .iter()
            .filter_map(|&ptr| {
                if ptr.is_null() {
                    None
                } else {
                    CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string())
                }
            })
            .collect()
    };

    if paths.is_empty() {
        return -1;
    }

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
    match client_guard.stage(&path_refs) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Get repository status as JSON
#[no_mangle]
pub extern "C" fn git_client_status(
    handle: GitClientHandle,
    out_json: *mut *mut c_char,
) -> c_int {
    if handle.is_null() || out_json.is_null() {
        return -1;
    }

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    match client_guard.status() {
        Ok(status) => {
            match serde_json::to_string(&status) {
                Ok(json) => {
                    let c_string = match CString::new(json) {
                        Ok(s) => s,
                        Err(_) => return -1,
                    };
                    unsafe {
                        *out_json = c_string.into_raw();
                    }
                    0
                }
                Err(_) => -1,
            }
        }
        Err(_) => -1,
    }
}

/// Get file statuses as JSON
#[no_mangle]
pub extern "C" fn git_client_file_statuses(
    handle: GitClientHandle,
    out_json: *mut *mut c_char,
) -> c_int {
    if handle.is_null() || out_json.is_null() {
        return -1;
    }

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    match client_guard.file_statuses() {
        Ok(statuses) => {
            match serde_json::to_string(&statuses) {
                Ok(json) => {
                    let c_string = match CString::new(json) {
                        Ok(s) => s,
                        Err(_) => return -1,
                    };
                    unsafe {
                        *out_json = c_string.into_raw();
                    }
                    0
                }
                Err(_) => -1,
            }
        }
        Err(_) => -1,
    }
}

/// Get branches as JSON
#[no_mangle]
pub extern "C" fn git_client_branches(
    handle: GitClientHandle,
    out_json: *mut *mut c_char,
) -> c_int {
    if handle.is_null() || out_json.is_null() {
        return -1;
    }

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    match client_guard.branches() {
        Ok(branches) => {
            match serde_json::to_string(&branches) {
                Ok(json) => {
                    let c_string = match CString::new(json) {
                        Ok(s) => s,
                        Err(_) => return -1,
                    };
                    unsafe {
                        *out_json = c_string.into_raw();
                    }
                    0
                }
                Err(_) => -1,
            }
        }
        Err(_) => -1,
    }
}

/// Checkout branch
#[no_mangle]
pub extern "C" fn git_client_checkout_branch(
    handle: GitClientHandle,
    name: *const c_char,
    create: c_int,
) -> c_int {
    if handle.is_null() || name.is_null() {
        return -1;
    }

    let name = unsafe {
        match CStr::from_ptr(name).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return -1,
        }
    };

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    match client_guard.checkout_branch(&name, create != 0) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Create branch
#[no_mangle]
pub extern "C" fn git_client_create_branch(
    handle: GitClientHandle,
    name: *const c_char,
    start_point: *const c_char,
) -> c_int {
    if handle.is_null() || name.is_null() {
        return -1;
    }

    let name = unsafe {
        match CStr::from_ptr(name).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return -1,
        }
    };

    let start_point = unsafe {
        if start_point.is_null() {
            None
        } else {
            CStr::from_ptr(start_point).to_str().ok().map(|s| s.to_string())
        }
    };

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    match client_guard.create_branch(&name, start_point.as_deref()) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Get commit log as JSON
#[no_mangle]
pub extern "C" fn git_client_log(
    handle: GitClientHandle,
    branch: *const c_char,
    limit: c_int,
    out_json: *mut *mut c_char,
) -> c_int {
    if handle.is_null() || out_json.is_null() {
        return -1;
    }

    let branch = unsafe {
        if branch.is_null() {
            None
        } else {
            CStr::from_ptr(branch).to_str().ok().map(|s| s.to_string())
        }
    };

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    match client_guard.log(branch.as_deref(), limit as usize) {
        Ok(commits) => {
            match serde_json::to_string(&commits) {
                Ok(json) => {
                    let c_string = match CString::new(json) {
                        Ok(s) => s,
                        Err(_) => return -1,
                    };
                    unsafe {
                        *out_json = c_string.into_raw();
                    }
                    0
                }
                Err(_) => -1,
            }
        }
        Err(_) => -1,
    }
}

/// Get diff as JSON
#[no_mangle]
pub extern "C" fn git_client_diff_workdir(
    handle: GitClientHandle,
    out_json: *mut *mut c_char,
) -> c_int {
    if handle.is_null() || out_json.is_null() {
        return -1;
    }

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    match client_guard.diff_workdir() {
        Ok(diff) => {
            match serde_json::to_string(&diff) {
                Ok(json) => {
                    let c_string = match CString::new(json) {
                        Ok(s) => s,
                        Err(_) => return -1,
                    };
                    unsafe {
                        *out_json = c_string.into_raw();
                    }
                    0
                }
                Err(_) => -1,
            }
        }
        Err(_) => -1,
    }
}

/// Get stashes as JSON
#[no_mangle]
pub extern "C" fn git_client_stash_list(
    handle: GitClientHandle,
    out_json: *mut *mut c_char,
) -> c_int {
    if handle.is_null() || out_json.is_null() {
        return -1;
    }

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    match client_guard.stash_list() {
        Ok(stashes) => {
            match serde_json::to_string(&stashes) {
                Ok(json) => {
                    let c_string = match CString::new(json) {
                        Ok(s) => s,
                        Err(_) => return -1,
                    };
                    unsafe {
                        *out_json = c_string.into_raw();
                    }
                    0
                }
                Err(_) => -1,
            }
        }
        Err(_) => -1,
    }
}

/// Save stash
#[no_mangle]
pub extern "C" fn git_client_stash_save(
    handle: GitClientHandle,
    message: *const c_char,
    include_untracked: c_int,
) -> c_int {
    if handle.is_null() {
        return -1;
    }

    let message = unsafe {
        if message.is_null() {
            None
        } else {
            CStr::from_ptr(message).to_str().ok().map(|s| s.to_string())
        }
    };

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    match client_guard.stash_save(message.as_deref(), include_untracked != 0) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Pop stash
#[no_mangle]
pub extern "C" fn git_client_stash_pop(handle: GitClientHandle, index: c_int) -> c_int {
    if handle.is_null() {
        return -1;
    }

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    match client_guard.stash_pop(index as usize) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Get remotes as JSON
#[no_mangle]
pub extern "C" fn git_client_remotes(
    handle: GitClientHandle,
    out_json: *mut *mut c_char,
) -> c_int {
    if handle.is_null() || out_json.is_null() {
        return -1;
    }

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    match client_guard.remotes() {
        Ok(remotes) => {
            match serde_json::to_string(&remotes) {
                Ok(json) => {
                    let c_string = match CString::new(json) {
                        Ok(s) => s,
                        Err(_) => return -1,
                    };
                    unsafe {
                        *out_json = c_string.into_raw();
                    }
                    0
                }
                Err(_) => -1,
            }
        }
        Err(_) => -1,
    }
}

/// Add remote
#[no_mangle]
pub extern "C" fn git_client_add_remote(
    handle: GitClientHandle,
    name: *const c_char,
    url: *const c_char,
) -> c_int {
    if handle.is_null() || name.is_null() || url.is_null() {
        return -1;
    }

    let name = unsafe {
        match CStr::from_ptr(name).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return -1,
        }
    };

    let url = unsafe {
        match CStr::from_ptr(url).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return -1,
        }
    };

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    match client_guard.add_remote(&name, &url) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Get tags as JSON
#[no_mangle]
pub extern "C" fn git_client_tags(
    handle: GitClientHandle,
    out_json: *mut *mut c_char,
) -> c_int {
    if handle.is_null() || out_json.is_null() {
        return -1;
    }

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    match client_guard.tags() {
        Ok(tags) => {
            match serde_json::to_string(&tags) {
                Ok(json) => {
                    let c_string = match CString::new(json) {
                        Ok(s) => s,
                        Err(_) => return -1,
                    };
                    unsafe {
                        *out_json = c_string.into_raw();
                    }
                    0
                }
                Err(_) => -1,
            }
        }
        Err(_) => -1,
    }
}

/// Create tag
#[no_mangle]
pub extern "C" fn git_client_create_tag(
    handle: GitClientHandle,
    name: *const c_char,
    target: *const c_char,
    message: *const c_char,
) -> c_int {
    if handle.is_null() || name.is_null() {
        return -1;
    }

    let name = unsafe {
        match CStr::from_ptr(name).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return -1,
        }
    };

    let target = unsafe {
        if target.is_null() {
            None
        } else {
            CStr::from_ptr(target).to_str().ok().map(|s| s.to_string())
        }
    };

    let message = unsafe {
        if message.is_null() {
            None
        } else {
            CStr::from_ptr(message).to_str().ok().map(|s| s.to_string())
        }
    };

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    match client_guard.create_tag(&name, target.as_deref(), message.as_deref()) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Free a string returned by the library
#[no_mangle]
pub extern "C" fn git_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

/// Get submodules as JSON
#[no_mangle]
pub extern "C" fn git_client_submodules(
    handle: GitClientHandle,
    out_json: *mut *mut c_char,
) -> c_int {
    if handle.is_null() || out_json.is_null() {
        return -1;
    }

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    match client_guard.submodules() {
        Ok(submodules) => {
            match serde_json::to_string(&submodules) {
                Ok(json) => {
                    let c_string = match CString::new(json) {
                        Ok(s) => s,
                        Err(_) => return -1,
                    };
                    unsafe {
                        *out_json = c_string.into_raw();
                    }
                    0
                }
                Err(_) => -1,
            }
        }
        Err(_) => -1,
    }
}

/// Get blame for file as JSON
#[no_mangle]
pub extern "C" fn git_client_blame(
    handle: GitClientHandle,
    path: *const c_char,
    out_json: *mut *mut c_char,
) -> c_int {
    if handle.is_null() || path.is_null() || out_json.is_null() {
        return -1;
    }

    let path = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return -1,
        }
    };

    let client = unsafe { &*handle };
    let client_guard = client.client.lock().unwrap();

    match client_guard.blame(&path, None) {
        Ok(blame) => {
            match serde_json::to_string(&blame) {
                Ok(json) => {
                    let c_string = match CString::new(json) {
                        Ok(s) => s,
                        Err(_) => return -1,
                    };
                    unsafe {
                        *out_json = c_string.into_raw();
                    }
                    0
                }
                Err(_) => -1,
            }
        }
        Err(_) => -1,
    }
}
