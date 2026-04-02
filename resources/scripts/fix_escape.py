import os
path = 'C:/Users/z7299/Documents/GitHub/AnixOps-EasySSH/core/src/database_client/drivers/mod.rs'
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

# Fix the broken escape sequences - the issue is "\\"") should be "\\\"")
content = content.replace('"\\\\""),', '"\\\\\\\"),')

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
print('Fixed')
