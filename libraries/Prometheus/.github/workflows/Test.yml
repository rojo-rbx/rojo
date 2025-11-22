name: Test
on:  
  push:
  pull_request:
    branches:
      - master

jobs:
  test-linux:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout repo
        uses: actions/checkout@master
      - name: Install Lua
        uses: leafo/gh-actions-lua@master
        with:
          luaVersion: 5.1
      - name: Run test case
        run: lua ./tests.lua --Linux --CI
