import json
import asyncio
from contextlib import contextmanager
import obsws_python as obs

conn_param = {
    "host": "localhost",
    "port": 4455,
    "password": "kTUby09Oaay5PEKI",
}

WORKSPACE_TO_SCENE = {
    "1": "screensaver",
}

DEFAULT_SCENE = "screencast"

STOP_SIGNAL = asyncio.Event()


@contextmanager
def obs_client():
    with obs.ReqClient(**conn_param) as client:
        yield client


def switch_scene(client, scene_name):
    print(f"Switching to scene: {scene_name}")
    res = client.set_current_program_scene(scene_name)
    print(f"RESULT: {res}")


async def listen_for_workspaces():
    proc = await asyncio.create_subprocess_exec(
        "swaymsg",
        "-t",
        "subscribe",
        "-m",
        '["workspace"]',
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )

    with obs_client() as client:
        while not STOP_SIGNAL.is_set():
            line = await proc.stdout.readline()
            if line:
                try:
                    data = json.loads(line.decode("utf-8"))
                    if data.get("change") == "focus":
                        current_workspace = data["current"]["name"]
                        print(f"Current workspace: {current_workspace}")
                        scene = WORKSPACE_TO_SCENE.get(current_workspace, DEFAULT_SCENE)
                        switch_scene(client, scene)
                except Exception as e:
                    print(f"⚠️ JSON parse error: {e}")


if __name__ == "__main__":
    try:
        asyncio.run(listen_for_workspaces())
    except KeyboardInterrupt:
        STOP_SIGNAL.set()
        print("Shutting down...")
