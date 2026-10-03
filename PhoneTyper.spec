# -*- mode: python ; coding: utf-8 -*-

from PyInstaller.utils.hooks import collect_data_files, collect_submodules


a = Analysis(
    ['server.py'],
    pathex=[],
    binaries=[],
    datas=[('web', 'web')] + collect_data_files('customtkinter'),
    hiddenimports=['tray', 'pystray._win32'] + collect_submodules('customtkinter'),
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=['unittest', 'pydoc', 'doctest', 'tkinter.test', 'test', 'pdb', 'pydoc_data', 'numpy', 'numpy.libs', 'scipy', 'scipy.libs'],
    noarchive=False,
    optimize=0,
)
pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name='PhoneTyper',
    debug=False,
    bootloader_ignore_signals=False,
    strip=True,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    icon=['web/icons/icon.ico'],
)
