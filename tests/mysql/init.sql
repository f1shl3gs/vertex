-- 1. Native Password (SHA1)
CREATE USER 'user_native'@'%' IDENTIFIED WITH mysql_native_password BY 'password';

-- 2. Caching SHA2 (8.0 默认, 带缓存)
CREATE USER 'user_caching'@'%' IDENTIFIED WITH caching_sha2_password BY 'password';

-- 3. SHA256 Password (仅 RSA)
CREATE USER 'user_sha256'@'%' IDENTIFIED WITH sha256_password BY 'password';

-- 4. Cleartext (通常用于 LDAP/Proxy，这里用 native 存储但在客户端强制明文传输)
CREATE USER 'user_clear'@'%' IDENTIFIED WITH mysql_native_password BY 'password';

GRANT ALL PRIVILEGES ON *.* TO 'user_native'@'%';
GRANT ALL PRIVILEGES ON *.* TO 'user_caching'@'%';
GRANT ALL PRIVILEGES ON *.* TO 'user_sha256'@'%';
GRANT ALL PRIVILEGES ON *.* TO 'user_clear'@'%';

FLUSH PRIVILEGES;