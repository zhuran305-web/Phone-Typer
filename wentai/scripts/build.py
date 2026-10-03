#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""文台 App 一键构建：工具链检测 -> 生成图标 -> Gradle 出包 -> 拷贝 APK 到 dist/。

用法:
    python scripts/build.py

约束:
    - 所有产物只允许落在 wentai/ 目录内，APK 集中输出到 wentai/dist/
    - 工具链缺失时中止构建并输出缺失清单与安装指引
"""
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DIST_DIR = ROOT / "dist"
ICON_FILE = ROOT / "wentai-icon-fg-1024.png"
GEN_ICONS = ROOT / "tools" / "gen_icons.py"
WRAPPER_JAR = ROOT / "gradle" / "wrapper" / "gradle-wrapper.jar"

MIN_JDK = 17
COMPILE_SDK = 34


def find_java():
    """返回可用的 java 可执行文件路径，未找到返回 None。"""
    java_home = os.environ.get("JAVA_HOME")
    if java_home:
        candidate = Path(java_home) / "bin" / ("java.exe" if os.name == "nt" else "java")
        if candidate.is_file():
            return str(candidate)

    which = shutil.which("java")
    if which:
        return which
    return None


def java_major_version(java_exe):
    """解析 java -version 的主版本号（如 17、21），失败返回 None。"""
    try:
        result = subprocess.run(
            [java_exe, "-version"],
            capture_output=True,
            text=True,
            timeout=30,
        )
    except Exception:  # noqa: BLE001
        return None

    text = (result.stderr or "") + (result.stdout or "")
    match = re.search(r'version "(\d+)(?:\.(\d+))?', text)
    if not match:
        return None
    major = int(match.group(1))
    # 旧式版本号 1.8 -> 8
    if major == 1 and match.group(2):
        return int(match.group(2))
    return major


def find_android_sdk():
    """按 ANDROID_HOME -> ANDROID_SDK_ROOT -> 默认安装路径 查找 SDK。"""
    for env_name in ("ANDROID_HOME", "ANDROID_SDK_ROOT"):
        value = os.environ.get(env_name)
        if value and Path(value).is_dir():
            return Path(value)

    candidates = [
        Path.home() / "AppData" / "Local" / "Android" / "Sdk",
        Path.home() / "Library" / "Android" / "sdk",
        Path.home() / "Android" / "Sdk",
    ]
    for candidate in candidates:
        if candidate.is_dir():
            return candidate
    return None


def find_gradle_command():
    """优先使用工程内 Gradle Wrapper，其次使用系统 Gradle。"""
    if WRAPPER_JAR.is_file():
        script = "gradlew.bat" if os.name == "nt" else "gradlew"
        wrapper = ROOT / script
        if wrapper.is_file():
            return [str(wrapper)]

    system_gradle = shutil.which("gradle")
    if system_gradle:
        return [system_gradle]
    return None


def collect_problems():
    problems = []

    java_exe = find_java()
    if not java_exe:
        problems.append("未检测到 JDK（需 JDK 17+）：请安装并设置 JAVA_HOME")
    else:
        major = java_major_version(java_exe)
        if major is None:
            problems.append(f"无法识别 JDK 版本：{java_exe}")
        elif major < MIN_JDK:
            problems.append(f"JDK 版本过低（当前 {major}），需 >= {MIN_JDK}")

    sdk = find_android_sdk()
    if not sdk:
        problems.append("未检测到 Android SDK：请设置 ANDROID_HOME / ANDROID_SDK_ROOT")
    else:
        platform = sdk / "platforms" / f"android-{COMPILE_SDK}"
        if not platform.is_dir():
            problems.append(
                f"Android SDK 缺少平台 android-{COMPILE_SDK}"
                f"（sdkmanager \"platforms;android-{COMPILE_SDK}\"）"
            )
        build_tools = sdk / "build-tools"
        if not build_tools.is_dir() or not any(build_tools.iterdir()):
            problems.append("Android SDK 缺少 build-tools（sdkmanager \"build-tools;34.0.0\"）")

    if find_gradle_command() is None:
        problems.append(
            "未检测到 Gradle：请安装 Gradle 8.7，或生成 Wrapper"
            "（gradle wrapper --gradle-version 8.7）"
        )

    return problems


def print_guide():
    print("安装指引：")
    print("  1. 安装 JDK 17+（如 Eclipse Temurin），并设置 JAVA_HOME")
    print("  2. 安装 Android SDK（Android Studio 或 cmdline-tools），并设置 ANDROID_HOME")
    print(f"     需要 platforms;android-{COMPILE_SDK} 与 build-tools")
    print("  3. 安装 Gradle 8.7，或在工程根目录执行: gradle wrapper --gradle-version 8.7")
    print("  4. 完成后重新运行 build.bat")


def build_env(sdk):
    env = dict(os.environ)
    if sdk is not None:
        env["ANDROID_HOME"] = str(sdk)
        env["ANDROID_SDK_ROOT"] = str(sdk)
    return env


def main() -> int:
    print("== 文台 App 构建 ==")

    if not ICON_FILE.is_file():
        print(f"[ERROR] 未找到图标素材：{ICON_FILE}")
        return 1

    problems = collect_problems()
    if problems:
        print("\n[构建中止] 工具链不完整，缺失清单：")
        for item in problems:
            print(f"  - {item}")
        print()
        print_guide()
        return 2

    sdk = find_android_sdk()
    env = build_env(sdk)

    print("\n[1/3] 生成图标 ...")
    result = subprocess.run([sys.executable, str(GEN_ICONS)], cwd=str(ROOT), env=env)
    if result.returncode != 0:
        print("[ERROR] 图标生成失败，构建中止。")
        return result.returncode

    print("\n[2/3] Gradle 构建 release ...")
    command = find_gradle_command() + ["copyReleaseApk", "--no-daemon"]
    result = subprocess.run(command, cwd=str(ROOT), env=env)
    if result.returncode != 0:
        print("\n[ERROR] Gradle 构建失败，未产出 APK。请查看上方日志定位问题。")
        return result.returncode

    print("\n[3/3] 校验产物 ...")
    apks = sorted(DIST_DIR.glob("*.apk"))
    if not apks:
        print(f"[ERROR] 构建完成但未在 {DIST_DIR} 找到 APK。")
        return 1

    print(f"\n构建成功：{apks[-1]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
