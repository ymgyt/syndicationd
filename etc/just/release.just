import '.base.just'

# Release synd_auth
[no-cd]
auth *flags:
	just changelog auth
	cargo release --package synd-auth {{ flags }}

# Release synd_o11y
[no-cd]
o11y *flags:
	just changelog o11y
	cargo release --package synd-o11y {{ flags }}

# Release synd_feed
[no-cd]
feed *flags:
	just changelog feed
	cargo release --package synd-feed {{ flags }}

# Release synd_term
[no-cd]
term *flags:
	just changelog term
	cargo release --package synd-term {{ flags }}

# Release synd_api
[no-cd]
api *flags:
	just changelog api
	cargo release --package synd-api {{ flags }}

