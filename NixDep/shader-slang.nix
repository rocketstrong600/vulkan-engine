{
  lib,
  stdenv,
  fetchFromGitHub,
  cmake,
  ninja,
  python3,
  miniz,
  libxml2,
  libX11,
  # spirv-headers,
  # glslang,
  versionCheckHook,
  gitUpdater,

  # Required for compiling to SPIR-V or GLSL
  withGlslang ? true,
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "shader-slang";
  version = "2025.6.1";

  src = fetchFromGitHub {
    owner = "shader-slang";
    repo = "slang";
    tag = "v${finalAttrs.version}";
    hash = "sha256-yNPAJX7OxxQLXDm3s7Hx5QA9fxy1qbAMp4LKYVqxMVM=";
    fetchSubmodules = true;
  };

  patches = [
    #./1-find-packages.patch
  ];

  outputs = [
    "out"
    "dev"
    "doc"
  ];

  strictDeps = true;

  nativeBuildInputs = [
    cmake
    ninja
    python3
  ];

  buildInputs =
    [
      miniz
      libxml2
    ]
    ++ (lib.optionals stdenv.hostPlatform.isLinux [
      libX11
    ])
    ++ (lib.optionals withGlslang [
      # SPIRV-tools is included in glslang.
      # glslang
    ]);

  separateDebugInfo = true;

  # Required for spaces in cmakeFlags, see https://github.com/NixOS/nixpkgs/issues/114044
  __structuredAttrs = true;

  preConfigure =
    lib.optionalString stdenv.hostPlatform.isLinux ''
      # required to handle LTO objects
      export AR="${stdenv.cc.targetPrefix}gcc-ar"
      export NM="${stdenv.cc.targetPrefix}gcc-nm"
      export RANLIB="${stdenv.cc.targetPrefix}gcc-ranlib"
    ''
    + ''
      # cmake setup hook only sets CMAKE_AR and CMAKE_RANLIB, but not these
      prependToVar cmakeFlags "-DCMAKE_CXX_COMPILER_AR=$(command -v $AR)"
      prependToVar cmakeFlags "-DCMAKE_CXX_COMPILER_RANLIB=$(command -v $RANLIB)"
    '';

  cmakeFlags =
    [
      "-GNinja Multi-Config"
      # The cmake setup hook only specifies `-DCMAKE_BUILD_TYPE=Release`,
      # which does nothing for "Ninja Multi-Config".
      "-DCMAKE_CONFIGURATION_TYPES=RelWithDebInfo"
      # Handled by separateDebugInfo so we don't need special installation handling
      "-DSLANG_ENABLE_SPLIT_DEBUG_INFO=OFF"
      "-DSLANG_VERSION_FULL=v${finalAttrs.version}-nixpkgs"
      # slang-rhi tries to download WebGPU dawn binaries, and as stated on
      # https://github.com/shader-slang/slang-rhi is "under active refactoring
      # and development, and is not yet ready for general use."
      "-DSLANG_ENABLE_SLANG_RHI=OFF"
      "-DSLANG_SLANG_LLVM_FLAVOR=DISABLE"
    ];

  nativeInstallCheckInputs = [ versionCheckHook ];
  versionCheckProgram = "${placeholder "out"}/bin/slangc";
  versionCheckProgramArg = [ "-v" ];
  doInstallCheck = true;

  passthru.updateScript = gitUpdater {
    rev-prefix = "v";
    ignoredVersions = "*-draft";
  };

  meta = {
    description = "A shading language that makes it easier to build and maintain large shader codebases in a modular and extensible fashion";
    homepage = "https://github.com/shader-slang/slang";
    license = lib.licenses.asl20-llvm;
    maintainers = with lib.maintainers; [ niklaskorz ];
    mainProgram = "slangc";
    platforms = lib.platforms.all;
  };
})
