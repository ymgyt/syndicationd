import '.base.just'

# Generate synd_auth CHANGELOG
[no-cd]
auth:
    GIT_CLIFF__GIT__TAG_PATTERN="synd-auth-v.*" \
    git cliff --include-path "{{auth_dir}}/**" --include-path "crates/synd_authn/**" out> {{auth_dir}}/CHANGELOG.md

# Generate synd_o11y CHANGELOG
[no-cd]
o11y:
    GIT_CLIFF__GIT__TAG_PATTERN="synd-o11y-v.*" \
    git cliff --include-path "{{o11y_dir}}/**" out> {{o11y_dir}}/CHANGELOG.md

# Generate synd_feed CHANGELOG
[no-cd]
feed:
    GIT_CLIFF__GIT__TAG_PATTERN="synd-feed-v.*" \
    git cliff --include-path "{{feed_dir}}/**" out> {{feed_dir}}/CHANGELOG.md

# Generate synd_term CHANGELOG
[no-cd]
term:
    GIT_CLIFF__GIT__TAG_PATTERN="synd-term-v.*" \
    git cliff --include-path "{{term_dir}}/**" out> {{term_dir}}/CHANGELOG.md

# Generate synd-api CHANGELOG
[no-cd]
api:
    GIT_CLIFF__GIT__TAG_PATTERN="synd-api-v.*" \
    git cliff --include-path "{{api_dir}}/**" out> {{api_dir}}/CHANGELOG.md
