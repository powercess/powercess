#!/bin/sh
set -e

# 运行时环境变量配置
# 默认值
API_BASE="${NUXT_PUBLIC_API_BASE:-http://localhost:5090}"
WS_BASE="${NUXT_PUBLIC_WS_BASE:-ws://localhost:5090}"

# 替换所有 JS 文件中的占位符
find /usr/share/nginx/html -name "*.js" -type f -exec sed -i \
    -e "s|__NUXT_PUBLIC_API_BASE__|${API_BASE}|g" \
    -e "s|__NUXT_PUBLIC_WS_BASE__|${WS_BASE}|g" {} \;

# 替换 HTML 文件中的占位符
find /usr/share/nginx/html -name "*.html" -type f -exec sed -i \
    -e "s|__NUXT_PUBLIC_API_BASE__|${API_BASE}|g" \
    -e "s|__NUXT_PUBLIC_WS_BASE__|${WS_BASE}|g" {} \;

exec "$@"