<# 
打包 `semantic-search-mcp` + `resources/`，生成可上传 GitHub Release 的归档文件（Windows）。

产物：
- dist/semantic-search-mcp-windows-x86_64.zip

说明：
- 运行时默认会优先从“可执行文件同级的 resources/”加载模型与 onnxruntime；
  若你希望自定义资源目录，可设置环境变量 `SEMANTIC_SEARCH_RESOURCES_DIR`。
#>

$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

$BinName = "semantic-search-mcp"

Write-Host "[package] building $BinName (release)"
cargo build --release --bin $BinName

$OutDir = Join-Path $Root "dist"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$Stage = Join-Path ([System.IO.Path]::GetTempPath()) ("mcp_pkg_" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $Stage | Out-Null

$PkgDir = Join-Path $Stage "$BinName-windows-x86_64"
New-Item -ItemType Directory -Force -Path $PkgDir | Out-Null

Copy-Item -Force (Join-Path $Root "target\release\$BinName.exe") (Join-Path $PkgDir "$BinName.exe")
Copy-Item -Recurse -Force (Join-Path $Root "resources") (Join-Path $PkgDir "resources")

if (Test-Path (Join-Path $Root "README.md")) {
  Copy-Item -Force (Join-Path $Root "README.md") (Join-Path $PkgDir "README.md")
}
if (Test-Path (Join-Path $Root "README.zh-CN.md")) {
  Copy-Item -Force (Join-Path $Root "README.zh-CN.md") (Join-Path $PkgDir "README.zh-CN.md")
}

$ZipPath = Join-Path $OutDir "$BinName-windows-x86_64.zip"
if (Test-Path $ZipPath) { Remove-Item -Force $ZipPath }

Write-Host "[package] creating zip $ZipPath"
Compress-Archive -Path $PkgDir -DestinationPath $ZipPath

Remove-Item -Recurse -Force $Stage

Write-Host "[package] wrote $ZipPath"

