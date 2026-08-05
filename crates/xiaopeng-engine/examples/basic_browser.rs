use xiaopeng_engine::{EngineConfig, BrowserEngine};

fn main() {

    // Create a basic HTML document with our newly supported Grid and Flexbox features
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        body {
            margin: 0;
            padding: 20px;
            background-color: #f0f0f0;
        }
        .header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            background-color: #333;
            color: white;
            padding: 10px 20px;
            font-size: 24px;
        }
        .grid-container {
            display: grid;
            grid-template-columns: 200px 1fr 200px;
            gap: 20px;
            margin-top: 20px;
        }
        .sidebar {
            background-color: #fff;
            padding: 15px;
            border: 1px solid #ccc;
        }
        .main-content {
            background-color: #fff;
            padding: 20px;
            border: 1px solid #ccc;
        }
        .btn {
            background-color: #007bff;
            color: white;
            padding: 10px 15px;
            border-width: 0;
            font-size: 16px;
        }
    </style>
</head>
<body>
    <div class="header">
        <div>XiaopengBrowser</div>
        <div>v0.4.0</div>
    </div>
    <div class="grid-container">
        <div class="sidebar">
            <div>Menu Item 1</div>
            <div>Menu Item 2</div>
            <div>Menu Item 3</div>
        </div>
        <div class="main-content">
            <div style="font-size: 32px; color: #333;">Welcome to the engine!</div>
            <div style="margin-top: 20px; color: #666;">
                This example demonstrates our CSS Grid and Flexbox capabilities, 
                as well as our SIMD rendering and GPU acceleration.
            </div>
            <div class="btn" style="margin-top: 20px;" id="click-me">Click Me!</div>
        </div>
        <div class="sidebar">
            <div>Ads / Widgets</div>
        </div>
    </div>
    
    <script>
        document.getElementById('click-me').addEventListener('click', function() {
            console.log("Button clicked!");
            // Incremental layout will be triggered if we change attributes
        });
    </script>
</body>
</html>
"#;

    let config = EngineConfig {
        width: 1024,
        height: 768,
        title: "XiaopengBrowser Example".to_string(),
        headless: false,
        headless_output: None,
    };

    let mut engine = BrowserEngine::new(config);
    engine.load_html(html);
    engine.run();
}
