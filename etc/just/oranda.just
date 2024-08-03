import '.base.just'

# Build and serve oranda server then watch
[no-cd]
dev:
    oranda dev --verbose

# Generate oranda github actions workflow
[no-cd]
generate:
	oranda generate ci --output-path .github/workflows/website.yaml --ci github
	echo "Make sure add `- run: cp etc/website/CNAME public/CNAME` to workflow"

