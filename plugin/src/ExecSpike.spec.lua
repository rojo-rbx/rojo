return function()
	local ExecSpike = require(script.Parent.ExecSpike)
	local helpers = ExecSpike._test

	local function countPlainOccurrences(haystack, needle)
		local count = 0
		local position = 1

		while true do
			local first, last = string.find(haystack, needle, position, true)
			if first == nil then
				return count
			end

			count += 1
			position = last + 1
		end
	end

	local function makeDependencies()
		local state = {
			cleanupCount = 0,
			configureCount = 0,
			editCheckCount = 0,
			finishCount = 0,
			invocationCount = 0,
			result = nil,
		}
		local dependencies = {
			isEdit = function()
				state.editCheckCount += 1
				return true
			end,
			generateUniqueToken = function()
				return "test-token"
			end,
			createTemporaryModule = function()
				return {}
			end,
			configureTemporaryModule = function(temporaryModule, name, wrapper)
				state.configureCount += 1
				temporaryModule.name = name
				temporaryModule.wrapper = wrapper
			end,
			requireTemporaryModule = function()
				return function()
					state.invocationCount += 1
					return state.result
				end
			end,
			tryBeginRecording = function()
				return "recording"
			end,
			finishRecording = function()
				state.finishCount += 1
			end,
			cleanupTemporaryModule = function()
				state.cleanupCount += 1
			end,
			traceback = function(message)
				return "traceback: " .. message
			end,
		}

		return dependencies, state
	end

	describe("ExecSpike wrapper", function()
		it("contains submitted source exactly once", function()
			local source = "local uniqueMarker = 123\nreturn uniqueMarker"
			local wrapper = helpers.buildWrapper(source)

			expect(countPlainOccurrences(wrapper, source)).to.equal(1)
		end)

		it("places top-level return inside the returned function", function()
			local wrapper = helpers.buildWrapper("return 17")

			expect(wrapper:find("return function(rojoExec)", 1, true)).to.equal(1)
			expect(wrapper:find("\nreturn 17\nend\n", 1, true)).to.be.ok()
		end)
	end)

	describe("ExecSpike names", function()
		it("sanitizes unsafe and control characters", function()
			local sanitized = helpers.sanitizeScriptName("bad/name\\with\0controls\n:and spaces")

			expect(sanitized:find("[^%w%._%-]")).to.equal(nil)
			expect(#sanitized > 0).to.equal(true)
		end)

		it("creates a unique name for every call", function()
			local first = helpers.makeTemporaryName("test.lua", "same-token")
			local second = helpers.makeTemporaryName("test.lua", "same-token")

			expect(first).never.to.equal(second)
			expect(first:find("[^%w%._%-]")).to.equal(nil)
			expect(second:find("[^%w%._%-]")).to.equal(nil)
		end)
	end)

	describe("ExecSpike result validation", function()
		it("accepts nil", function()
			local ok = helpers.validateResult(nil)

			expect(ok).to.equal(true)
		end)

		it("accepts strings", function()
			local ok = helpers.validateResult("result")

			expect(ok).to.equal(true)
		end)

		it("accepts numbers", function()
			local ok = helpers.validateResult(42)

			expect(ok).to.equal(true)
		end)

		it("accepts booleans", function()
			local ok = helpers.validateResult(false)

			expect(ok).to.equal(true)
		end)

		it("rejects unsupported values", function()
			local ok, errorMessage = helpers.validateResult({})

			expect(ok).to.equal(false)
			expect(errorMessage:find("table", 1, true)).to.be.ok()
		end)
	end)

	describe("ExecSpike controller", function()
		it("includes a traceback for runtime failures", function()
			local dependencies = makeDependencies()
			dependencies.requireTemporaryModule = function()
				return function()
					error("intentional runtime failure")
				end
			end

			local result = helpers.runWithDependencies("return true", "runtime.lua", dependencies)

			expect(result.ok).to.equal(false)
			expect(result.phase).to.equal("runtime")
			expect(result.error:find("intentional runtime failure", 1, true)).to.be.ok()
			expect(result.traceback:find("traceback:", 1, true)).to.be.ok()
		end)

		it("runs cleanup exactly once", function()
			local cleanupCount = 0
			local cleanupOnce = helpers.makeProtectedOnce(function()
				cleanupCount += 1
			end)

			cleanupOnce()
			cleanupOnce()

			expect(cleanupCount).to.equal(1)
		end)

		it("finishes a recording exactly once", function()
			local dependencies, state = makeDependencies()

			local result = helpers.runWithDependencies("return nil", "success.lua", dependencies)

			expect(result.ok).to.equal(true)
			expect(state.finishCount).to.equal(1)
			expect(state.cleanupCount).to.equal(1)
		end)

		it("does not invoke user code when recording cannot start", function()
			local dependencies, state = makeDependencies()
			dependencies.tryBeginRecording = function()
				return nil
			end

			local result = helpers.runWithDependencies("return true", "blocked.lua", dependencies)

			expect(result.ok).to.equal(false)
			expect(result.phase).to.equal("internal")
			expect(state.invocationCount).to.equal(0)
			expect(state.finishCount).to.equal(0)
			expect(state.cleanupCount).to.equal(1)
		end)

		it("rejects a mode change immediately before invocation", function()
			local dependencies, state = makeDependencies()
			dependencies.isEdit = function()
				state.editCheckCount += 1
				return state.editCheckCount < 3
			end

			local result = helpers.runWithDependencies("return true", "mode-change.lua", dependencies)

			expect(result.ok).to.equal(false)
			expect(result.phase).to.equal("rejected")
			expect(state.invocationCount).to.equal(0)
			expect(state.finishCount).to.equal(1)
			expect(state.cleanupCount).to.equal(1)
		end)

		it("classifies require failures as compile failures and cleans up", function()
			local dependencies, state = makeDependencies()
			dependencies.requireTemporaryModule = function()
				error("expected compile failure")
			end

			local result = helpers.runWithDependencies("local =", "compile.lua", dependencies)

			expect(result.ok).to.equal(false)
			expect(result.phase).to.equal("compile")
			expect(result.error:find("expected compile failure", 1, true)).to.be.ok()
			expect(state.invocationCount).to.equal(0)
			expect(state.finishCount).to.equal(0)
			expect(state.cleanupCount).to.equal(1)
		end)

		it("cleans up after an unexpected module configuration failure", function()
			local dependencies, state = makeDependencies()
			dependencies.configureTemporaryModule = function()
				error("Source assignment was rejected")
			end

			local result = helpers.runWithDependencies("return true", "configure.lua", dependencies)

			expect(result.ok).to.equal(false)
			expect(result.phase).to.equal("internal")
			expect(result.error:find("Source assignment was rejected", 1, true)).to.be.ok()
			expect(state.invocationCount).to.equal(0)
			expect(state.finishCount).to.equal(0)
			expect(state.cleanupCount).to.equal(1)
		end)
	end)
end
