use crate::types::*;
use std::collections::HashMap;

pub struct CollectionManager {
    collections: HashMap<String, Collection>,
}

impl CollectionManager {
    pub fn new() -> Self {
        Self {
            collections: HashMap::new(),
        }
    }

    pub fn add_collection(&mut self, collection: Collection) {
        self.collections.insert(collection.id.clone(), collection);
    }

    pub fn get_collection(&self, id: &str) -> Option<&Collection> {
        self.collections.get(id)
    }

    pub fn get_collection_mut(&mut self, id: &str) -> Option<&mut Collection> {
        self.collections.get_mut(id)
    }

    pub fn remove_collection(&mut self, id: &str) -> Option<Collection> {
        self.collections.remove(id)
    }

    pub fn list_collections(&self) -> Vec<&Collection> {
        let mut cols: Vec<_> = self.collections.values().collect();
        cols.sort_by(|a, b| a.name.cmp(&b.name));
        cols
    }

    pub fn find_request(&self, request_id: &str) -> Option<(&Collection, &ApiRequest)> {
        for collection in self.collections.values() {
            // Check root requests
            for request in &collection.requests {
                if request.id == request_id {
                    return Some((collection, request));
                }
            }

            // Check folders
            for folder in &collection.folders {
                if let Some(result) = Self::find_request_in_folder(folder, request_id) {
                    return Some((collection, result));
                }
            }
        }
        None
    }

    pub fn find_request_mut(&mut self, request_id: &str) -> Option<(&mut Collection, &mut ApiRequest)> {
        for collection in self.collections.values_mut() {
            // Check root requests
            for i in 0..collection.requests.len() {
                if collection.requests[i].id == request_id {
                    let ptr = &mut collection.requests[i] as *mut ApiRequest;
                    return Some((collection, unsafe { &mut *ptr }));
                }
            }

            // Check folders
            for folder in &mut collection.folders {
                if let Some(result) = Self::find_request_in_folder_mut(folder, request_id) {
                    let ptr = result as *mut ApiRequest;
                    return Some((collection, unsafe { &mut *ptr }));
                }
            }
        }
        None
    }

    fn find_request_in_folder<'a>(folder: &'a CollectionFolder, request_id: &str) -> Option<&'a ApiRequest> {
        for request in &folder.requests {
            if request.id == request_id {
                return Some(request);
            }
        }

        for sub_folder in &folder.folders {
            if let Some(result) = Self::find_request_in_folder(sub_folder, request_id) {
                return Some(result);
            }
        }

        None
    }

    fn find_request_in_folder_mut<'a>(folder: &'a mut CollectionFolder, request_id: &str) -> Option<&'a mut ApiRequest> {
        for i in 0..folder.requests.len() {
            if folder.requests[i].id == request_id {
                return Some(&mut folder.requests[i]);
            }
        }

        for sub_folder in &mut folder.folders {
            if let Some(result) = Self::find_request_in_folder_mut(sub_folder, request_id) {
                return Some(result);
            }
        }

        None
    }

    pub fn add_request_to_collection(
        &mut self,
        collection_id: &str,
        request: ApiRequest,
        folder_id: Option<&str>,
    ) -> ApiResult<()> {
        let collection = self.collections
            .get_mut(collection_id)
            .ok_or_else(|| ApiError::Database("Collection not found".to_string()))?;

        if let Some(folder_id) = folder_id {
            // Add to folder
            for folder in &mut collection.folders {
                if Self::add_request_to_folder(folder, folder_id, request.clone()) {
                    return Ok(());
                }
            }
            Err(ApiError::Database("Folder not found".to_string()))
        } else {
            // Add to root
            collection.requests.push(request);
            collection.updated_at = chrono::Utc::now();
            Ok(())
        }
    }

    fn add_request_to_folder(folder: &mut CollectionFolder, folder_id: &str, request: ApiRequest) -> bool {
        if folder.id == folder_id {
            folder.requests.push(request);
            return true;
        }

        for sub_folder in &mut folder.folders {
            if Self::add_request_to_folder(sub_folder, folder_id, request.clone()) {
                return true;
            }
        }

        false
    }

    pub fn create_folder(
        &mut self,
        collection_id: &str,
        parent_folder_id: Option<&str>,
        name: impl Into<String>,
        description: Option<String>,
    ) -> ApiResult<CollectionFolder> {
        let folder = CollectionFolder {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description,
            requests: Vec::new(),
            folders: Vec::new(),
        };

        let collection = self.collections
            .get_mut(collection_id)
            .ok_or_else(|| ApiError::Database("Collection not found".to_string()))?;

        if let Some(parent_id) = parent_folder_id {
            // Add to parent folder
            for f in &mut collection.folders {
                if Self::add_subfolder(f, parent_id, &folder) {
                    collection.updated_at = chrono::Utc::now();
                    return Ok(folder);
                }
            }
            Err(ApiError::Database("Parent folder not found".to_string()))
        } else {
            // Add to root
            collection.folders.push(folder.clone());
            collection.updated_at = chrono::Utc::now();
            Ok(folder)
        }
    }

    fn add_subfolder(folder: &mut CollectionFolder, parent_id: &str, new_folder: &CollectionFolder) -> bool {
        if folder.id == parent_id {
            folder.folders.push(new_folder.clone());
            return true;
        }

        for sub_folder in &mut folder.folders {
            if Self::add_subfolder(sub_folder, parent_id, new_folder) {
                return true;
            }
        }

        false
    }

    pub fn update_request(&mut self, request: ApiRequest) -> ApiResult<()> {
        if let Some((collection, existing)) = self.find_request_mut(&request.id) {
            *existing = request;
            collection.updated_at = chrono::Utc::now();
            Ok(())
        } else {
            Err(ApiError::Database("Request not found".to_string()))
        }
    }

    pub fn delete_request(&mut self, request_id: &str) -> ApiResult<()> {
        for collection in self.collections.values_mut() {
            // Check root requests
            let initial_len = collection.requests.len();
            collection.requests.retain(|r| r.id != request_id);
            if collection.requests.len() < initial_len {
                collection.updated_at = chrono::Utc::now();
                return Ok(());
            }

            // Check folders
            for folder in &mut collection.folders {
                if Self::delete_request_from_folder(folder, request_id) {
                    collection.updated_at = chrono::Utc::now();
                    return Ok(());
                }
            }
        }

        Err(ApiError::Database("Request not found".to_string()))
    }

    fn delete_request_from_folder(folder: &mut CollectionFolder, request_id: &str) -> bool {
        let initial_len = folder.requests.len();
        folder.requests.retain(|r| r.id != request_id);
        if folder.requests.len() < initial_len {
            return true;
        }

        for sub_folder in &mut folder.folders {
            if Self::delete_request_from_folder(sub_folder, request_id) {
                return true;
            }
        }

        false
    }

    /// Search across all collections
    pub fn search(&self, query: &str) -> Vec<&ApiRequest> {
        let query = query.to_lowercase();
        let mut results = Vec::new();

        for collection in self.collections.values() {
            // Search root requests
            for request in &collection.requests {
                if self.request_matches(request, &query) {
                    results.push(request);
                }
            }

            // Search folders
            for folder in &collection.folders {
                self.search_folder(folder, &query, &mut results);
            }
        }

        results
    }

    fn search_folder<'a>(&self, folder: &'a CollectionFolder, query: &str, results: &mut Vec<&'a ApiRequest>) {
        for request in &folder.requests {
            if self.request_matches(request, query) {
                results.push(request);
            }
        }

        for sub_folder in &folder.folders {
            self.search_folder(sub_folder, query, results);
        }
    }

    fn request_matches(&self, request: &ApiRequest, query: &str) -> bool {
        request.name.to_lowercase().contains(query)
            || request.url.to_lowercase().contains(query)
            || request.method.to_string().to_lowercase().contains(query)
    }

    /// Get all requests flattened
    pub fn get_all_requests(&self) -> Vec<&ApiRequest> {
        let mut all = Vec::new();

        for collection in self.collections.values() {
            for request in &collection.requests {
                all.push(request);
            }

            for folder in &collection.folders {
                self.collect_requests_from_folder(folder, &mut all);
            }
        }

        all
    }

    fn collect_requests_from_folder<'a>(&self, folder: &'a CollectionFolder, results: &mut Vec<&'a ApiRequest>) {
        for request in &folder.requests {
            results.push(request);
        }

        for sub_folder in &folder.folders {
            self.collect_requests_from_folder(sub_folder, results);
        }
    }

    /// Duplicate a request
    pub fn duplicate_request(&mut self, request_id: &str) -> ApiResult<ApiRequest> {
        let original = self.find_request(request_id)
            .map(|(_, r)| r.clone())
            .ok_or_else(|| ApiError::Database("Request not found".to_string()))?;

        let mut duplicate = original;
        duplicate.id = uuid::Uuid::new_v4().to_string();
        duplicate.name = format!("{} (Copy)", duplicate.name);
        duplicate.created_at = chrono::Utc::now();
        duplicate.updated_at = chrono::Utc::now();

        // Find where to add the duplicate
        for collection in self.collections.values_mut() {
            for (i, request) in collection.requests.iter().enumerate() {
                if request.id == request_id {
                    collection.requests.insert(i + 1, duplicate.clone());
                    return Ok(duplicate);
                }
            }

            for folder in &mut collection.folders {
                if Self::insert_duplicate_in_folder(folder, request_id, &duplicate) {
                    return Ok(duplicate);
                }
            }
        }

        Err(ApiError::Database("Failed to duplicate request".to_string()))
    }

    fn insert_duplicate_in_folder(folder: &mut CollectionFolder, request_id: &str, duplicate: &ApiRequest) -> bool {
        for (i, request) in folder.requests.iter().enumerate() {
            if request.id == request_id {
                folder.requests.insert(i + 1, duplicate.clone());
                return true;
            }
        }

        for sub_folder in &mut folder.folders {
            if Self::insert_duplicate_in_folder(sub_folder, request_id, duplicate) {
                return true;
            }
        }

        false
    }
}

impl Default for CollectionManager {
    fn default() -> Self {
        Self::new()
    }
}
