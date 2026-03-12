#!/bin/bash
# Feature flag matrix testing script
# This demonstrates the feature matrix testing implementation

set -e

echo "🧪 Feature Flag Matrix Testing"
echo "==============================="

# Test minimal configuration (no features)
echo "Testing minimal configuration (no default features)..."
cargo test --test feature_matrix --no-default-features

# Test individual output features
echo "Testing individual output features..."
cargo test --test feature_matrix --no-default-features --features="output-json"
cargo test --test feature_matrix --no-default-features --features="output-junit" 
cargo test --test feature_matrix --no-default-features --features="libtest"

# Test individual observability features  
echo "Testing individual observability features..."
cargo test --test feature_matrix --no-default-features --features="timestamps"
cargo test --test feature_matrix --no-default-features --features="tracing"
cargo test --test feature_matrix --no-default-features --features="observability"

# Test output format combinations
echo "Testing output format combinations..."
cargo test --test feature_matrix --no-default-features --features="output-json,output-junit"
cargo test --test feature_matrix --no-default-features --features="output-json,libtest"
cargo test --test feature_matrix --no-default-features --features="output-junit,libtest"

# Test observability combinations
echo "Testing observability combinations..."
cargo test --test feature_matrix --no-default-features --features="timestamps,tracing"
cargo test --test feature_matrix --no-default-features --features="timestamps,observability"
cargo test --test feature_matrix --no-default-features --features="tracing,observability"

# Test cross-category combinations
echo "Testing cross-category combinations..."
cargo test --test feature_matrix --no-default-features --features="output-json,timestamps"
cargo test --test feature_matrix --no-default-features --features="output-junit,tracing"
cargo test --test feature_matrix --no-default-features --features="libtest,observability"

# Test with macros enabled
echo "Testing with macros enabled..."
cargo test --test feature_matrix --no-default-features --features="macros"
cargo test --test feature_matrix --no-default-features --features="macros,output-json"

# Test comprehensive combinations
echo "Testing comprehensive combinations..."
cargo test --test feature_matrix --no-default-features --features="output-json,output-junit,timestamps,tracing"
cargo test --test feature_matrix --no-default-features --features="output-json,output-junit,libtest,observability"

# Test full feature set
echo "Testing full feature matrix..."
cargo test --test feature_matrix --all-features

echo "✅ All feature combinations tested successfully!"
echo ""
echo "📊 Feature Matrix Summary:"
echo "- Minimal configuration: ✓"
echo "- Individual features: ✓"
echo "- Output combinations: ✓" 
echo "- Observability combinations: ✓"
echo "- Cross-category combinations: ✓"
echo "- Full feature matrix: ✓"
echo ""
echo "🎯 Feature matrix validation complete!"