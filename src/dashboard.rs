use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    response::Html,
    routing::get,
};

use crate::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/", get(dashboard_page))
}

async fn dashboard_page(State(state): State<Arc<AppState>>) -> Html<String> {
    let stats = state.storage.get_stats();
    let buckets = state.storage.list_buckets();
    let port = state.config.port;

    Html(render_dashboard(port, &stats, &buckets))
}

fn render_dashboard(
    port: u16,
    stats: &crate::models::StorageStats,
    buckets: &[crate::models::Bucket],
) -> String {
    let bucket_cards: String = buckets
        .iter()
        .map(|b| {
            format!(
                r#"
                <div class="bucket-card" onclick="openBucket('{name}')">
                    <div class="bucket-card-header">
                        <div class="bucket-icon">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <path d="M2 7V17C2 19 4 21 8 21H16C20 21 22 19 22 17V7"/>
                                <path d="M2 7L5 3H19L22 7"/>
                                <path d="M2 7H22"/>
                                <path d="M9 11H15"/>
                            </svg>
                        </div>
                        <button class="btn-icon delete-btn" onclick="event.stopPropagation(); deleteBucket('{name}')" title="Delete bucket">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <path d="M3 6h18M8 6V4a2 2 0 012-2h4a2 2 0 012 2v2M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6"/>
                                <path d="M10 11v6M14 11v6"/>
                            </svg>
                        </button>
                    </div>
                    <h3 class="bucket-name">{name}</h3>
                    <div class="bucket-meta">
                        <span class="meta-item">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
                                <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
                                <path d="M14 2v6h6"/>
                            </svg>
                            {count} objects
                        </span>
                        <span class="meta-item">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
                                <path d="M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 002 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0022 16z"/>
                            </svg>
                            {size}
                        </span>
                    </div>
                    <div class="bucket-region">{region}</div>
                </div>"#,
                name = b.name,
                count = b.object_count,
                size = crate::storage::human_readable_size(b.total_size),
                region = b.region
            )
        })
        .collect();

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>FreeBucket — Local Storage Dashboard</title>
    <meta name="description" content="FreeBucket: A local S3-compatible object storage service dashboard">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
    <style>
        :root {{
            --bg-primary: #0a0e1a;
            --bg-secondary: #111827;
            --bg-card: #1a1f35;
            --bg-card-hover: #222845;
            --bg-input: #151b2e;
            --border-color: #2a3152;
            --border-hover: #3d4a7a;
            --text-primary: #e8ecf4;
            --text-secondary: #8892a8;
            --text-muted: #5a6580;
            --accent-primary: #6366f1;
            --accent-primary-hover: #818cf8;
            --accent-glow: rgba(99, 102, 241, 0.3);
            --accent-secondary: #06b6d4;
            --accent-success: #10b981;
            --accent-warning: #f59e0b;
            --accent-danger: #ef4444;
            --accent-danger-hover: #f87171;
            --gradient-primary: linear-gradient(135deg, #6366f1, #8b5cf6, #06b6d4);
            --gradient-card: linear-gradient(145deg, rgba(26,31,53,0.9), rgba(17,24,39,0.95));
            --shadow-sm: 0 1px 3px rgba(0,0,0,0.3);
            --shadow-md: 0 4px 16px rgba(0,0,0,0.4);
            --shadow-lg: 0 8px 32px rgba(0,0,0,0.5);
            --shadow-glow: 0 0 20px var(--accent-glow);
            --radius-sm: 8px;
            --radius-md: 12px;
            --radius-lg: 16px;
            --radius-xl: 20px;
        }}

        * {{ margin:0; padding:0; box-sizing:border-box; }}

        body {{
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
            background: var(--bg-primary);
            color: var(--text-primary);
            min-height: 100vh;
            overflow-x: hidden;
        }}

        /* Animated background */
        body::before {{
            content: '';
            position: fixed;
            top: 0; left: 0; right: 0; bottom: 0;
            background:
                radial-gradient(ellipse 80% 50% at 20% 20%, rgba(99,102,241,0.08), transparent),
                radial-gradient(ellipse 60% 40% at 80% 80%, rgba(6,182,212,0.06), transparent),
                radial-gradient(ellipse 50% 50% at 50% 50%, rgba(139,92,246,0.04), transparent);
            pointer-events: none;
            z-index: 0;
        }}

        /* Header */
        .header {{
            background: rgba(17,24,39,0.8);
            backdrop-filter: blur(20px);
            border-bottom: 1px solid var(--border-color);
            padding: 0 2rem;
            height: 64px;
            display: flex;
            align-items: center;
            justify-content: space-between;
            position: sticky;
            top: 0;
            z-index: 100;
        }}

        .logo {{
            display: flex;
            align-items: center;
            gap: 12px;
        }}

        .logo-icon {{
            width: 36px;
            height: 36px;
            border-radius: var(--radius-sm);
            background: var(--gradient-primary);
            display: flex;
            align-items: center;
            justify-content: center;
            box-shadow: var(--shadow-glow);
        }}

        .logo-icon svg {{
            width: 20px;
            height: 20px;
            color: white;
        }}

        .logo-text {{
            font-size: 1.25rem;
            font-weight: 700;
            background: var(--gradient-primary);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
        }}

        .logo-badge {{
            font-size: 0.65rem;
            padding: 2px 8px;
            border-radius: 20px;
            background: rgba(99,102,241,0.15);
            color: var(--accent-primary-hover);
            font-weight: 600;
            letter-spacing: 0.5px;
            text-transform: uppercase;
        }}

        .header-actions {{
            display: flex;
            align-items: center;
            gap: 12px;
        }}

        /* Main Content */
        .main {{
            position: relative;
            z-index: 1;
            max-width: 1400px;
            margin: 0 auto;
            padding: 2rem;
        }}

        /* Stats Cards */
        .stats-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
            gap: 1.25rem;
            margin-bottom: 2rem;
        }}

        .stat-card {{
            background: var(--gradient-card);
            border: 1px solid var(--border-color);
            border-radius: var(--radius-lg);
            padding: 1.5rem;
            transition: all 0.3s ease;
        }}

        .stat-card:hover {{
            border-color: var(--border-hover);
            transform: translateY(-2px);
            box-shadow: var(--shadow-md);
        }}

        .stat-label {{
            font-size: 0.8rem;
            color: var(--text-muted);
            text-transform: uppercase;
            letter-spacing: 1px;
            font-weight: 600;
            margin-bottom: 0.5rem;
        }}

        .stat-value {{
            font-size: 2rem;
            font-weight: 800;
            background: var(--gradient-primary);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
        }}

        .stat-sub {{
            font-size: 0.8rem;
            color: var(--text-secondary);
            margin-top: 0.25rem;
        }}

        /* Section */
        .section {{
            margin-bottom: 2rem;
        }}

        .section-header {{
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 1.25rem;
        }}

        .section-title {{
            font-size: 1.35rem;
            font-weight: 700;
            color: var(--text-primary);
        }}

        /* Buttons */
        .btn {{
            display: inline-flex;
            align-items: center;
            gap: 8px;
            padding: 10px 20px;
            border: none;
            border-radius: var(--radius-sm);
            font-size: 0.875rem;
            font-weight: 600;
            font-family: inherit;
            cursor: pointer;
            transition: all 0.2s ease;
        }}

        .btn-primary {{
            background: var(--gradient-primary);
            color: white;
            box-shadow: var(--shadow-sm);
        }}

        .btn-primary:hover {{
            transform: translateY(-1px);
            box-shadow: var(--shadow-glow);
        }}

        .btn-secondary {{
            background: var(--bg-card);
            color: var(--text-primary);
            border: 1px solid var(--border-color);
        }}

        .btn-secondary:hover {{
            border-color: var(--border-hover);
            background: var(--bg-card-hover);
        }}

        .btn-danger {{
            background: var(--accent-danger);
            color: white;
        }}

        .btn-danger:hover {{
            background: var(--accent-danger-hover);
        }}

        .btn-icon {{
            width: 32px;
            height: 32px;
            display: flex;
            align-items: center;
            justify-content: center;
            border: none;
            border-radius: var(--radius-sm);
            background: transparent;
            color: var(--text-muted);
            cursor: pointer;
            transition: all 0.2s ease;
        }}

        .btn-icon:hover {{
            background: rgba(255,255,255,0.06);
            color: var(--text-primary);
        }}

        .delete-btn:hover {{
            color: var(--accent-danger);
            background: rgba(239,68,68,0.1);
        }}

        .btn-icon svg {{
            width: 16px;
            height: 16px;
        }}

        /* Bucket Grid */
        .bucket-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
            gap: 1.25rem;
        }}

        .bucket-card {{
            background: var(--gradient-card);
            border: 1px solid var(--border-color);
            border-radius: var(--radius-lg);
            padding: 1.5rem;
            cursor: pointer;
            transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
            position: relative;
            overflow: hidden;
        }}

        .bucket-card::before {{
            content: '';
            position: absolute;
            top: 0;
            left: 0;
            right: 0;
            height: 3px;
            background: var(--gradient-primary);
            opacity: 0;
            transition: opacity 0.3s ease;
        }}

        .bucket-card:hover {{
            border-color: var(--border-hover);
            transform: translateY(-4px);
            box-shadow: var(--shadow-lg);
        }}

        .bucket-card:hover::before {{
            opacity: 1;
        }}

        .bucket-card-header {{
            display: flex;
            align-items: flex-start;
            justify-content: space-between;
            margin-bottom: 1rem;
        }}

        .bucket-icon {{
            width: 44px;
            height: 44px;
            border-radius: var(--radius-md);
            background: rgba(99,102,241,0.12);
            display: flex;
            align-items: center;
            justify-content: center;
            color: var(--accent-primary);
        }}

        .bucket-icon svg {{
            width: 22px;
            height: 22px;
        }}

        .bucket-name {{
            font-size: 1.1rem;
            font-weight: 700;
            color: var(--text-primary);
            margin-bottom: 0.75rem;
            font-family: 'JetBrains Mono', monospace;
        }}

        .bucket-meta {{
            display: flex;
            gap: 1rem;
            margin-bottom: 0.5rem;
        }}

        .meta-item {{
            display: flex;
            align-items: center;
            gap: 6px;
            font-size: 0.8rem;
            color: var(--text-secondary);
        }}

        .bucket-region {{
            font-size: 0.75rem;
            color: var(--text-muted);
            display: inline-flex;
            align-items: center;
            padding: 3px 10px;
            border-radius: 20px;
            background: rgba(6,182,212,0.1);
            color: var(--accent-secondary);
            margin-top: 0.5rem;
        }}

        /* Empty State */
        .empty-state {{
            text-align: center;
            padding: 4rem 2rem;
            border: 2px dashed var(--border-color);
            border-radius: var(--radius-xl);
            background: rgba(17,24,39,0.3);
        }}

        .empty-icon {{
            width: 72px;
            height: 72px;
            margin: 0 auto 1.5rem;
            border-radius: var(--radius-lg);
            background: rgba(99,102,241,0.08);
            display: flex;
            align-items: center;
            justify-content: center;
            color: var(--text-muted);
        }}

        .empty-icon svg {{
            width: 36px;
            height: 36px;
        }}

        .empty-title {{
            font-size: 1.25rem;
            font-weight: 600;
            color: var(--text-secondary);
            margin-bottom: 0.5rem;
        }}

        .empty-desc {{
            font-size: 0.9rem;
            color: var(--text-muted);
            margin-bottom: 1.5rem;
        }}

        /* Modal */
        .modal-overlay {{
            display: none;
            position: fixed;
            top: 0; left: 0; right: 0; bottom: 0;
            background: rgba(0,0,0,0.7);
            backdrop-filter: blur(4px);
            z-index: 1000;
            align-items: center;
            justify-content: center;
        }}

        .modal-overlay.active {{
            display: flex;
        }}

        .modal {{
            background: var(--bg-secondary);
            border: 1px solid var(--border-color);
            border-radius: var(--radius-xl);
            padding: 2rem;
            min-width: 420px;
            max-width: 600px;
            width: 90%;
            box-shadow: var(--shadow-lg);
            animation: modalIn 0.3s cubic-bezier(0.4, 0, 0.2, 1);
        }}

        @keyframes modalIn {{
            from {{ opacity: 0; transform: scale(0.95) translateY(10px); }}
            to {{ opacity: 1; transform: scale(1) translateY(0); }}
        }}

        .modal-title {{
            font-size: 1.25rem;
            font-weight: 700;
            margin-bottom: 1.5rem;
        }}

        .form-group {{
            margin-bottom: 1.25rem;
        }}

        .form-label {{
            display: block;
            font-size: 0.8rem;
            font-weight: 600;
            color: var(--text-secondary);
            text-transform: uppercase;
            letter-spacing: 0.5px;
            margin-bottom: 0.5rem;
        }}

        .form-input {{
            width: 100%;
            padding: 12px 16px;
            background: var(--bg-input);
            border: 1px solid var(--border-color);
            border-radius: var(--radius-sm);
            color: var(--text-primary);
            font-size: 0.95rem;
            font-family: 'JetBrains Mono', monospace;
            transition: all 0.2s ease;
            outline: none;
        }}

        .form-input:focus {{
            border-color: var(--accent-primary);
            box-shadow: 0 0 0 3px var(--accent-glow);
        }}

        .form-hint {{
            font-size: 0.75rem;
            color: var(--text-muted);
            margin-top: 0.4rem;
        }}

        .modal-actions {{
            display: flex;
            justify-content: flex-end;
            gap: 0.75rem;
            margin-top: 1.5rem;
        }}

        /* Object Browser Modal */
        .object-browser {{
            min-width: 700px;
            max-width: 900px;
        }}

        .object-browser-header {{
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 1.5rem;
            padding-bottom: 1rem;
            border-bottom: 1px solid var(--border-color);
        }}

        .browser-title {{
            display: flex;
            align-items: center;
            gap: 12px;
        }}

        .browser-title h2 {{
            font-size: 1.2rem;
            font-weight: 700;
            font-family: 'JetBrains Mono', monospace;
        }}

        .object-list {{
            max-height: 400px;
            overflow-y: auto;
            border: 1px solid var(--border-color);
            border-radius: var(--radius-md);
        }}

        .object-list::-webkit-scrollbar {{
            width: 6px;
        }}

        .object-list::-webkit-scrollbar-track {{
            background: var(--bg-primary);
        }}

        .object-list::-webkit-scrollbar-thumb {{
            background: var(--border-color);
            border-radius: 3px;
        }}

        .object-row {{
            display: grid;
            grid-template-columns: 1fr 100px 150px 80px;
            gap: 1rem;
            align-items: center;
            padding: 0.85rem 1rem;
            border-bottom: 1px solid var(--border-color);
            transition: background 0.15s ease;
        }}

        .object-row:last-child {{
            border-bottom: none;
        }}

        .object-row:hover {{
            background: rgba(255,255,255,0.03);
        }}

        .object-row-header {{
            font-size: 0.75rem;
            font-weight: 600;
            color: var(--text-muted);
            text-transform: uppercase;
            letter-spacing: 0.5px;
            background: rgba(0,0,0,0.2);
        }}

        .object-row-header:hover {{
            background: rgba(0,0,0,0.2);
        }}

        .object-key {{
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.85rem;
            color: var(--text-primary);
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }}

        .object-size {{
            font-size: 0.8rem;
            color: var(--text-secondary);
            text-align: right;
        }}

        .object-date {{
            font-size: 0.8rem;
            color: var(--text-muted);
        }}

        .object-actions {{
            display: flex;
            justify-content: flex-end;
            gap: 4px;
        }}

        .empty-objects {{
            text-align: center;
            padding: 3rem 2rem;
            color: var(--text-muted);
        }}

        .empty-objects svg {{
            width: 40px;
            height: 40px;
            margin-bottom: 1rem;
            opacity: 0.4;
        }}

        /* Upload area */
        .upload-area {{
            border: 2px dashed var(--border-color);
            border-radius: var(--radius-md);
            padding: 2rem;
            text-align: center;
            margin-top: 1rem;
            transition: all 0.3s ease;
            cursor: pointer;
        }}

        .upload-area:hover,
        .upload-area.drag-over {{
            border-color: var(--accent-primary);
            background: rgba(99,102,241,0.05);
        }}

        .upload-area svg {{
            width: 32px;
            height: 32px;
            color: var(--text-muted);
            margin-bottom: 0.75rem;
        }}

        .upload-area p {{
            color: var(--text-secondary);
            font-size: 0.9rem;
        }}

        .upload-area .upload-hint {{
            color: var(--text-muted);
            font-size: 0.8rem;
            margin-top: 0.5rem;
        }}

        /* Toast Notifications */
        .toast-container {{
            position: fixed;
            bottom: 2rem;
            right: 2rem;
            z-index: 2000;
            display: flex;
            flex-direction: column;
            gap: 0.5rem;
        }}

        .toast {{
            padding: 1rem 1.5rem;
            border-radius: var(--radius-md);
            font-size: 0.9rem;
            font-weight: 500;
            color: white;
            box-shadow: var(--shadow-lg);
            animation: toastIn 0.3s ease, toastOut 0.3s ease 2.7s forwards;
            display: flex;
            align-items: center;
            gap: 10px;
            min-width: 300px;
        }}

        .toast.success {{
            background: linear-gradient(135deg, #059669, #10b981);
        }}

        .toast.error {{
            background: linear-gradient(135deg, #dc2626, #ef4444);
        }}

        .toast.info {{
            background: linear-gradient(135deg, #4f46e5, #6366f1);
        }}

        @keyframes toastIn {{
            from {{ opacity: 0; transform: translateX(100px); }}
            to {{ opacity: 1; transform: translateX(0); }}
        }}

        @keyframes toastOut {{
            from {{ opacity: 1; transform: translateX(0); }}
            to {{ opacity: 0; transform: translateX(100px); }}
        }}

        /* Responsive */
        @media (max-width: 768px) {{
            .main {{ padding: 1rem; }}
            .bucket-grid {{ grid-template-columns: 1fr; }}
            .stats-grid {{ grid-template-columns: repeat(2, 1fr); }}
            .modal {{ min-width: auto; }}
            .object-browser {{ min-width: auto; }}
            .object-row {{ grid-template-columns: 1fr 80px 60px; }}
            .object-date {{ display: none; }}
        }}

        /* Loading spinner */
        .spinner {{
            width: 20px;
            height: 20px;
            border: 2px solid rgba(255,255,255,0.3);
            border-top-color: white;
            border-radius: 50%;
            animation: spin 0.6s linear infinite;
        }}

        @keyframes spin {{
            to {{ transform: rotate(360deg); }}
        }}
    </style>
</head>
<body>
    <!-- Header -->
    <header class="header">
        <div class="logo">
            <div class="logo-icon">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M2 7V17C2 19 4 21 8 21H16C20 21 22 19 22 17V7"/>
                    <path d="M2 7L5 3H19L22 7"/>
                    <path d="M2 7H22"/>
                </svg>
            </div>
            <span class="logo-text">FreeBucket</span>
            <span class="logo-badge">Local</span>
        </div>
        <div class="header-actions">
            <button class="btn btn-secondary" onclick="location.reload()">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
                    <path d="M23 4v6h-6M1 20v-6h6"/>
                    <path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15"/>
                </svg>
                Refresh
            </button>
        </div>
    </header>

    <!-- Main Content -->
    <main class="main">
        <!-- Stats -->
        <div class="stats-grid">
            <div class="stat-card">
                <div class="stat-label">Total Buckets</div>
                <div class="stat-value" id="stat-buckets">{total_buckets}</div>
                <div class="stat-sub">Storage containers</div>
            </div>
            <div class="stat-card">
                <div class="stat-label">Total Objects</div>
                <div class="stat-value" id="stat-objects">{total_objects}</div>
                <div class="stat-sub">Files stored</div>
            </div>
            <div class="stat-card">
                <div class="stat-label">Storage Used</div>
                <div class="stat-value" id="stat-size">{total_size}</div>
                <div class="stat-sub">On local disk</div>
            </div>
            <div class="stat-card">
                <div class="stat-label">API Endpoint</div>
                <div class="stat-value" style="font-size:1rem; font-family:'JetBrains Mono',monospace;">:{port}</div>
                <div class="stat-sub">http://localhost:{port}/api</div>
            </div>
        </div>

        <!-- Buckets -->
        <div class="section">
            <div class="section-header">
                <h2 class="section-title">Buckets</h2>
                <button class="btn btn-primary" onclick="showCreateBucketModal()" id="create-bucket-btn">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
                        <path d="M12 5v14M5 12h14"/>
                    </svg>
                    Create Bucket
                </button>
            </div>
            <div id="bucket-list" class="bucket-grid">
                {bucket_cards}
            </div>
            {empty_state}
        </div>
    </main>

    <!-- Create Bucket Modal -->
    <div class="modal-overlay" id="create-modal">
        <div class="modal">
            <h3 class="modal-title">Create New Bucket</h3>
            <div class="form-group">
                <label class="form-label" for="bucket-name-input">Bucket Name</label>
                <input type="text" id="bucket-name-input" class="form-input"
                    placeholder="my-awesome-bucket" autocomplete="off"
                    pattern="[a-z0-9][a-z0-9.\-]{{2,62}}"
                    onkeydown="if(event.key==='Enter')createBucket()">
                <p class="form-hint">3–63 characters. Lowercase letters, numbers, hyphens, periods only.</p>
            </div>
            <div class="form-group">
                <label class="form-label" for="bucket-region-input">Region</label>
                <input type="text" id="bucket-region-input" class="form-input"
                    placeholder="local" value="local">
            </div>
            <div class="modal-actions">
                <button class="btn btn-secondary" onclick="closeModal('create-modal')">Cancel</button>
                <button class="btn btn-primary" onclick="createBucket()" id="create-confirm-btn">Create Bucket</button>
            </div>
        </div>
    </div>

    <!-- Object Browser Modal -->
    <div class="modal-overlay" id="browser-modal">
        <div class="modal object-browser">
            <div class="object-browser-header">
                <div class="browser-title">
                    <div class="bucket-icon" style="width:36px;height:36px;">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="18" height="18">
                            <path d="M2 7V17C2 19 4 21 8 21H16C20 21 22 19 22 17V7"/>
                            <path d="M2 7L5 3H19L22 7"/>
                            <path d="M2 7H22"/>
                        </svg>
                    </div>
                    <h2 id="browser-bucket-name"></h2>
                </div>
                <div style="display:flex;gap:8px;">
                    <button class="btn btn-primary" onclick="showUploadArea()" id="upload-btn">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
                            <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M17 8l-5-5-5 5M12 3v12"/>
                        </svg>
                        Upload
                    </button>
                    <button class="btn btn-secondary" onclick="closeModal('browser-modal')">Close</button>
                </div>
            </div>

            <!-- Upload Area -->
            <div id="upload-area" class="upload-area" style="display:none;"
                ondragover="event.preventDefault();this.classList.add('drag-over')"
                ondragleave="this.classList.remove('drag-over')"
                ondrop="handleDrop(event)"
                onclick="document.getElementById('file-input').click()">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M17 8l-5-5-5 5M12 3v12"/>
                </svg>
                <p>Drag & drop files here, or click to browse</p>
                <p class="upload-hint">Files will be uploaded to the current bucket</p>
                <input type="file" id="file-input" multiple style="display:none" onchange="handleFileSelect(event)">
            </div>

            <!-- Object List -->
            <div id="object-list-container">
                <div class="object-list">
                    <div class="object-row object-row-header">
                        <span>Key</span>
                        <span style="text-align:right">Size</span>
                        <span>Last Modified</span>
                        <span style="text-align:right">Actions</span>
                    </div>
                    <div id="object-list-body"></div>
                </div>
            </div>
        </div>
    </div>

    <!-- Toast Container -->
    <div class="toast-container" id="toasts"></div>

    <script>
        const API = '/api';
        let currentBucket = '';

        // ── Toast Notifications ─────────────────────────
        function toast(message, type = 'info') {{
            const container = document.getElementById('toasts');
            const el = document.createElement('div');
            el.className = 'toast ' + type;
            const icons = {{
                success: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="18" height="18"><path d="M20 6L9 17l-5-5"/></svg>',
                error: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="18" height="18"><circle cx="12" cy="12" r="10"/><path d="M15 9l-6 6M9 9l6 6"/></svg>',
                info: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="18" height="18"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/></svg>'
            }};
            el.innerHTML = (icons[type] || icons.info) + '<span>' + message + '</span>';
            container.appendChild(el);
            setTimeout(() => el.remove(), 3000);
        }}

        // ── Modal Helpers ───────────────────────────────
        function showModal(id) {{
            document.getElementById(id).classList.add('active');
        }}

        function closeModal(id) {{
            document.getElementById(id).classList.remove('active');
        }}

        // ── Bucket Operations ───────────────────────────
        function showCreateBucketModal() {{
            document.getElementById('bucket-name-input').value = '';
            document.getElementById('bucket-region-input').value = 'local';
            showModal('create-modal');
            setTimeout(() => document.getElementById('bucket-name-input').focus(), 100);
        }}

        async function createBucket() {{
            const name = document.getElementById('bucket-name-input').value.trim();
            const region = document.getElementById('bucket-region-input').value.trim() || 'local';

            if (!name) {{
                toast('Please enter a bucket name', 'error');
                return;
            }}

            try {{
                const res = await fetch(API + '/buckets', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ name, region }})
                }});

                if (!res.ok) {{
                    const err = await res.json();
                    toast(err.message || 'Failed to create bucket', 'error');
                    return;
                }}

                toast('Bucket "' + name + '" created successfully!', 'success');
                closeModal('create-modal');
                location.reload();
            }} catch (e) {{
                toast('Network error: ' + e.message, 'error');
            }}
        }}

        async function deleteBucket(name) {{
            if (!confirm('Are you sure you want to delete bucket "' + name + '"? This action cannot be undone.')) return;

            try {{
                const res = await fetch(API + '/buckets/' + encodeURIComponent(name), {{
                    method: 'DELETE'
                }});

                if (!res.ok) {{
                    const err = await res.json();
                    toast(err.message || 'Failed to delete bucket', 'error');
                    return;
                }}

                toast('Bucket "' + name + '" deleted', 'success');
                location.reload();
            }} catch (e) {{
                toast('Network error: ' + e.message, 'error');
            }}
        }}

        // ── Object Operations ───────────────────────────
        async function openBucket(name) {{
            currentBucket = name;
            document.getElementById('browser-bucket-name').textContent = name;
            document.getElementById('upload-area').style.display = 'none';
            showModal('browser-modal');
            await refreshObjects();
        }}

        async function refreshObjects() {{
            const body = document.getElementById('object-list-body');
            body.innerHTML = '<div class="empty-objects"><div class="spinner" style="margin:0 auto"></div></div>';

            try {{
                const res = await fetch(API + '/buckets/' + encodeURIComponent(currentBucket) + '/objects');
                if (!res.ok) throw new Error('Failed to load objects');

                const data = await res.json();
                if (!data.objects || data.objects.length === 0) {{
                    body.innerHTML = '<div class="empty-objects">' +
                        '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><path d="M14 2v6h6"/></svg>' +
                        '<p>No objects in this bucket</p>' +
                        '</div>';
                    return;
                }}

                body.innerHTML = data.objects.map(obj => {{
                    const size = humanSize(obj.size);
                    const date = new Date(obj.last_modified).toLocaleDateString();
                    return '<div class="object-row">' +
                        '<span class="object-key" title="' + escapeHtml(obj.key) + '">' + escapeHtml(obj.key) + '</span>' +
                        '<span class="object-size">' + size + '</span>' +
                        '<span class="object-date">' + date + '</span>' +
                        '<div class="object-actions">' +
                        '<button class="btn-icon" onclick="downloadObject(\'' + escapeHtml(obj.key) + '\')" title="Download">' +
                        '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M7 10l5 5 5-5M12 15V3"/></svg>' +
                        '</button>' +
                        '<button class="btn-icon delete-btn" onclick="deleteObject(\'' + escapeHtml(obj.key) + '\')" title="Delete">' +
                        '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 6h18M8 6V4a2 2 0 012-2h4a2 2 0 012 2v2M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6"/></svg>' +
                        '</button>' +
                        '</div></div>';
                }}).join('');
            }} catch (e) {{
                body.innerHTML = '<div class="empty-objects"><p>Error loading objects</p></div>';
                toast('Failed to load objects', 'error');
            }}
        }}

        async function downloadObject(key) {{
            const url = API + '/object/' + encodeURIComponent(currentBucket) + '/' + encodeURIComponent(key);
            const a = document.createElement('a');
            a.href = url;
            a.download = key.split('/').pop();
            document.body.appendChild(a);
            a.click();
            a.remove();
        }}

        async function deleteObject(key) {{
            if (!confirm('Delete object "' + key + '"?')) return;

            try {{
                const res = await fetch(API + '/object/' + encodeURIComponent(currentBucket) + '/' + encodeURIComponent(key), {{
                    method: 'DELETE'
                }});

                if (!res.ok) {{
                    toast('Failed to delete object', 'error');
                    return;
                }}

                toast('Object deleted', 'success');
                await refreshObjects();
            }} catch (e) {{
                toast('Network error: ' + e.message, 'error');
            }}
        }}

        // ── Upload ──────────────────────────────────────
        function showUploadArea() {{
            const area = document.getElementById('upload-area');
            area.style.display = area.style.display === 'none' ? 'block' : 'none';
        }}

        function handleDrop(event) {{
            event.preventDefault();
            event.currentTarget.classList.remove('drag-over');
            const files = event.dataTransfer.files;
            if (files.length > 0) uploadFiles(files);
        }}

        function handleFileSelect(event) {{
            const files = event.target.files;
            if (files.length > 0) uploadFiles(files);
            event.target.value = '';
        }}

        async function uploadFiles(files) {{
            const formData = new FormData();
            for (let i = 0; i < files.length; i++) {{
                formData.append('file', files[i]);
            }}

            try {{
                toast('Uploading ' + files.length + ' file(s)...', 'info');
                const res = await fetch(API + '/buckets/' + encodeURIComponent(currentBucket) + '/upload', {{
                    method: 'POST',
                    body: formData
                }});

                if (!res.ok) {{
                    toast('Upload failed', 'error');
                    return;
                }}

                const data = await res.json();
                toast(data.uploaded + ' file(s) uploaded successfully!', 'success');
                document.getElementById('upload-area').style.display = 'none';
                await refreshObjects();
            }} catch (e) {{
                toast('Upload error: ' + e.message, 'error');
            }}
        }}

        // ── Utilities ───────────────────────────────────
        function humanSize(bytes) {{
            const units = ['B', 'KB', 'MB', 'GB', 'TB'];
            let i = 0;
            let size = bytes;
            while (size >= 1024 && i < units.length - 1) {{
                size /= 1024;
                i++;
            }}
            return i === 0 ? bytes + ' B' : size.toFixed(1) + ' ' + units[i];
        }}

        function escapeHtml(str) {{
            const div = document.createElement('div');
            div.textContent = str;
            return div.innerHTML;
        }}

        // Close modals on overlay click
        document.querySelectorAll('.modal-overlay').forEach(overlay => {{
            overlay.addEventListener('click', (e) => {{
                if (e.target === overlay) {{
                    overlay.classList.remove('active');
                }}
            }});
        }});

        // Close modals on Escape
        document.addEventListener('keydown', (e) => {{
            if (e.key === 'Escape') {{
                document.querySelectorAll('.modal-overlay.active').forEach(m => m.classList.remove('active'));
            }}
        }});
    </script>
</body>
</html>"##,
        total_buckets = stats.total_buckets,
        total_objects = stats.total_objects,
        total_size = stats.total_size_human,
        port = port,
        bucket_cards = bucket_cards,
        empty_state = if buckets.is_empty() {
            r#"<div class="empty-state">
                <div class="empty-icon">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M2 7V17C2 19 4 21 8 21H16C20 21 22 19 22 17V7"/>
                        <path d="M2 7L5 3H19L22 7"/>
                        <path d="M2 7H22"/>
                        <path d="M12 11v6M9 14h6"/>
                    </svg>
                </div>
                <h3 class="empty-title">No buckets yet</h3>
                <p class="empty-desc">Create your first bucket to start storing objects</p>
                <button class="btn btn-primary" onclick="showCreateBucketModal()">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
                        <path d="M12 5v14M5 12h14"/>
                    </svg>
                    Create First Bucket
                </button>
            </div>"#
        } else {
            ""
        }
    )
}
