import re

# Read the file
with open('platforms/windows/easyssh-winui/src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Check if methods already exist
if 'fn start_edit_server' in content:
    print("start_edit_server method already exists")
else:
    # Find the add_server method end and insert start_edit_server and update_server after it
    add_server_end_pattern = r'(            Err\(e\) => \{\s*error!\("Failed to add server: \{\}", e\);\s*self\.add_error = Some\(format!\("Failed to add server: \{\}", e\)\);\s*\}\s*\}\s*\})'

    new_methods = '''\1

    fn start_edit_server(&mut self) {
        if let Some(server_id) = self.selected_server.as_ref() {
            if let Some(server) = self.servers.iter().find(|s| s.id == *server_id).cloned() {
                self.editing_server_id = Some(server.id.clone());
                self.edit_server.name = server.name;
                self.edit_server.host = server.host;
                self.edit_server.port = server.port.to_string();
                self.edit_server.username = server.username;
                self.edit_server.auth_type = match server.auth_type.as_str() {
                    "key" => AuthType::Key,
                    _ => AuthType::Password,
                };
                self.edit_server.group_id = server.group_id;
                self.edit_error = None;
                self.show_edit_dialog = true;
            }
        }
    }

    fn update_server(&mut self) {
        let server_id = match &self.editing_server_id {
            Some(id) => id.clone(),
            None => {
                self.edit_error = Some("No server selected for editing".to_string());
                return;
            }
        };

        if self.edit_server.name.is_empty() {
            self.edit_error = Some("Name is required".to_string());
            return;
        }
        if self.edit_server.host.is_empty() {
            self.edit_error = Some("Host is required".to_string());
            return;
        }
        if self.edit_server.username.is_empty() {
            self.edit_error = Some("Username is required".to_string());
            return;
        }

        let port: i64 = self.edit_server.port.parse().unwrap_or(22);
        let auth = match self.edit_server.auth_type {
            AuthType::Password => "password",
            AuthType::Key => "key",
        };

        let vm = self.view_model.lock().unwrap();
        match vm.update_server(
            &server_id,
            &self.edit_server.name,
            &self.edit_server.host,
            port,
            &self.edit_server.username,
            auth,
        ) {
            Ok(_) => {
                let server_name = self.edit_server.name.clone();
                info!("Server updated successfully: {}", server_name);
                self.show_edit_dialog = false;
                self.editing_server_id = None;
                self.edit_server = EditServerForm::default();
                self.edit_error = None;
                drop(vm);
                self.refresh_servers();
                info!("Server list refreshed after update, total servers: {}", self.servers.len());
                self.ux_manager.show_toast(
                    ToastNotification::success("服务器更新成功", &format!("服务器 '{}' 已更新", server_name))
                );
            }
            Err(e) => {
                error!("Failed to update server: {}", e);
                self.edit_error = Some(format!("Failed to update server: {}", e));
            }
        }
    }'''

    content = re.sub(add_server_end_pattern, new_methods, content, flags=re.DOTALL)
    print("Added start_edit_server and update_server methods")

# Write the file back
with open('platforms/windows/easyssh-winui/src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Done!")
