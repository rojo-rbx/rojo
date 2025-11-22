name: Build

on:
  push:
    branches:
      main

jobs:
  build:
    runs-on: windows-latest # gh-actions-lua doesn't work on windows
    steps:
      - name: Checkout repo
        uses: actions/checkout@master
      - name: Download srlua-mingw
        run: curl -o srlua.zip "https://raw.githubusercontent.com/joedf/LuaBuilds/gh-pages/hdata/srlua-5.1.5_Win32_bin.zip"
      - name: Unzip srlua-mingw
        run: |
          tar -xf srlua.zip
          Rename-Item -Path srglue.exe -NewName glue.exe
      - name: Download Lua53
        run: curl -o Lua53.zip "https://raw.githubusercontent.com/joedf/LuaBuilds/gh-pages/hdata/lua-5.3.5_Win64_bin.zip"
      - name: Unzip Lua53
        run: |
          tar -xf Lua53.zip
      - run: dir
      - name: Build the project
        run: |
          ./build.bat
        shell: bash
      - name: Zip the build
        uses: papeloto/action-zip@v1
        with:
          files: build/
          dest: build.zip
      - name: Load version and name
        run: | # I have no idea why but 5.1 just won't work for some reason
          echo "prometheus_full_version=$(./lua.exe ./src/config.lua --FullVersion)" >> $GITHUB_ENV
          echo "prometheus_version=$(./lua.exe ./src/config.lua --Version)" >> $GITHUB_ENV
        shell: bash
      - name: Upload binaries to release
        uses: svenstaro/upload-release-action@v2
        with:
          repo_token: ${{ secrets.GITHUB_TOKEN }}
          file: build.zip
          asset_name: ${{ env.prometheus_full_version }}.zip
          tag: release-${{ github.ref }}-${{ env.prometheus_version }}
          release_name: ${{ env.prometheus_version }}
          overwrite: true
          body: ${{ env.prometheus_full_version }}  ${{ github.event.commits[0].message }}