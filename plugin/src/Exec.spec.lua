return function()
	local Packages = script.Parent.Parent.Packages
	local Http = require(Packages.Http)
	local Promise = require(Packages.Promise)
	local Exec = require(script.Parent.Exec)
	local Types = require(script.Parent.Types)
	local helpers = Exec._test

	local function countPlainOccurrences(haystack, needle)
		local count = 0
		local position = 1
		while true do
			local first, last = haystack:find(needle, position, true)
			if first == nil then
				return count
			end
			count += 1
			position = last + 1
		end
	end

	local function makeRunDependencies()
		local state = {
			cleanupCount = 0,
			editCheckCount = 0,
			finishCount = 0,
			invocationCount = 0,
			loaded = function()
				return nil
			end,
			recording = "recording",
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
				temporaryModule.name = name
				temporaryModule.wrapper = wrapper
			end,
			requireTemporaryModule = function()
				return function(rojoExec)
					state.invocationCount += 1
					return state.loaded(rojoExec)
				end
			end,
			tryBeginRecording = function()
				return state.recording
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
			stringify = tostring,
			encodePayload = Http.msgpackEncode,
		}

		return dependencies, state
	end

	local function runWith(dependencies, source, scriptName)
		local job = {
			jobId = "00000000-0000-0000-0000-000000000000",
			scriptName = scriptName or "test.lua",
			source = source or "return nil",
			state = "claimed",
		}
		local controller = helpers.makeExecutionController(dependencies)
		return helpers.runJob(job, dependencies, controller)
	end

	local function makeScheduler()
		local queue = {}
		return {
			delay = function()
				return Promise.new(function(resolve)
					table.insert(queue, resolve)
				end)
			end,
			step = function()
				local resolve = table.remove(queue, 1)
				assert(resolve ~= nil, "no scheduled delay to resolve")
				resolve()
			end,
			count = function()
				return #queue
			end,
		}
	end

	local function claimedJob()
		return {
			jobId = "00000000-0000-0000-0000-000000000001",
			scriptName = "test.lua",
			source = "return true",
			state = "claimed",
		}
	end

	describe("Exec wrapper and names", function()
		it("contains submitted source exactly once", function()
			local source = "local uniqueMarker = 42\nreturn uniqueMarker"
			expect(countPlainOccurrences(helpers.buildWrapper(source), source)).to.equal(1)
		end)

		it("supports top-level return and shadows plugin", function()
			local wrapper = helpers.buildWrapper("return true")
			expect(wrapper:find("return function(rojoExec)", 1, true)).to.equal(1)
			expect(wrapper:find("local plugin = nil", 1, true)).to.be.ok()
			expect(wrapper:find("\nreturn true\nend\n", 1, true)).to.be.ok()
		end)

		it("sanitizes and uniquifies temporary names", function()
			local sanitized = helpers.sanitizeScriptName("bad/name\0\n with spaces")
			local first = helpers.makeTemporaryName(sanitized, "token")
			local second = helpers.makeTemporaryName(sanitized, "token")
			expect(sanitized:find("[^%w%._%-]")).to.equal(nil)
			expect(first).never.to.equal(second)
		end)
	end)

	describe("Exec result encoding", function()
		it("encodes nil", function()
			local encoded = helpers.encodeExecValue(nil)
			expect(encoded.kind).to.equal("nil")
		end)

		it("encodes strings", function()
			local encoded = helpers.encodeExecValue("hello")
			expect(encoded.kind).to.equal("string")
			expect(encoded.value).to.equal("hello")
		end)

		it("encodes numbers", function()
			local encoded = helpers.encodeExecValue(4.25)
			expect(encoded.kind).to.equal("number")
			expect(encoded.value).to.equal(4.25)
		end)

		it("encodes booleans", function()
			local encoded = helpers.encodeExecValue(false)
			expect(encoded.kind).to.equal("boolean")
			expect(encoded.value).to.equal(false)
		end)

		it("encodes dense arrays in order", function()
			local encoded = helpers.encodeExecValue({ "first", 2, true })
			expect(encoded.kind).to.equal("array")
			expect(encoded.value[1].kind).to.equal("string")
			expect(encoded.value[1].value).to.equal("first")
			expect(encoded.value[2].kind).to.equal("number")
			expect(encoded.value[2].value).to.equal(2)
			expect(encoded.value[3].kind).to.equal("boolean")
			expect(encoded.value[3].value).to.equal(true)
		end)

		it("sorts string-key tables deterministically", function()
			local encoded = helpers.encodeExecValue({ zed = 1, alpha = 2 })
			expect(encoded.kind).to.equal("table")
			expect(encoded.value[1].key).to.equal("alpha")
			expect(encoded.value[2].key).to.equal("zed")
		end)

		it("rejects cycles", function()
			local value = {}
			value.self = value
			local encoded, errorMessage = helpers.encodeExecValue(value)
			expect(encoded).to.equal(nil)
			expect(errorMessage:find("cycle", 1, true)).to.be.ok()
		end)

		it("rejects sparse arrays", function()
			local encoded, errorMessage = helpers.encodeExecValue({ [1] = true, [3] = true })
			expect(encoded).to.equal(nil)
			expect(errorMessage:find("sparse", 1, true)).to.be.ok()
		end)

		it("rejects mixed array and dictionary tables", function()
			local encoded, errorMessage = helpers.encodeExecValue({ [1] = true, named = true })
			expect(encoded).to.equal(nil)
			expect(errorMessage:find("mixes", 1, true)).to.be.ok()
		end)

		it("rejects non-string dictionary keys", function()
			local encoded, errorMessage = helpers.encodeExecValue({ [false] = true })
			expect(encoded).to.equal(nil)
			expect(errorMessage:find("key type", 1, true)).to.be.ok()
		end)

		it("rejects Instances", function()
			local instance = Instance.new("Folder")
			local encoded, errorMessage = helpers.encodeExecValue(instance)
			instance:Destroy()
			expect(encoded).to.equal(nil)
			expect(errorMessage:find("Instance", 1, true)).to.be.ok()
		end)

		it("rejects non-finite numbers", function()
			local nanEncoded, nanError = helpers.encodeExecValue(0 / 0)
			local infinityEncoded, infinityError = helpers.encodeExecValue(math.huge)
			expect(nanEncoded).to.equal(nil)
			expect(nanError:find("non-finite", 1, true)).to.be.ok()
			expect(infinityEncoded).to.equal(nil)
			expect(infinityError:find("non-finite", 1, true)).to.be.ok()
		end)
	end)

	describe("Exec logs", function()
		it("preserves print and warn sequence", function()
			local collector = helpers.makeLogCollector(tostring)
			collector:add("print", "hello", 2)
			collector:add("warn", "careful")
			collector:add("print", "done")
			local logs = collector:finish()
			expect(#logs).to.equal(3)
			expect(logs[1].level).to.equal("print")
			expect(logs[1].message).to.equal("hello\t2")
			expect(logs[2].level).to.equal("warn")
			expect(logs[2].message).to.equal("careful")
			expect(logs[3].level).to.equal("print")
			expect(logs[3].message).to.equal("done")
		end)

		it("bounds oversized entries and adds a warning", function()
			local collector = helpers.makeLogCollector(tostring)
			collector:add("print", string.rep("x", helpers.constants.maxLogEntryBytes + 100))
			local logs = collector:finish()
			expect(#logs).to.equal(2)
			expect(#logs[1].message <= helpers.constants.maxLogEntryBytes).to.equal(true)
			expect(logs[2].level).to.equal("warn")
		end)

		it("never exceeds the entry count limit", function()
			local collector = helpers.makeLogCollector(tostring)
			for index = 1, helpers.constants.maxLogEntries + 1 do
				collector:add("print", index)
			end
			local logs = collector:finish()
			expect(#logs).to.equal(helpers.constants.maxLogEntries)
			expect(logs[#logs].level).to.equal("warn")
		end)
	end)

	describe("Exec controller", function()
		it("posts a scalar success result", function()
			local dependencies, state = makeRunDependencies()
			state.loaded = function()
				return true
			end
			local payload = runWith(dependencies)
			expect(payload.outcome).to.equal("success")
			expect(payload.result.kind).to.equal("boolean")
			expect(payload.result.value).to.equal(true)
		end)

		it("lets the last explicit output override the return value", function()
			local dependencies, state = makeRunDependencies()
			state.loaded = function(rojoExec)
				rojoExec.output("first")
				rojoExec.output("last")
				return "returned"
			end
			local payload = runWith(dependencies)
			expect(payload.result.kind).to.equal("string")
			expect(payload.result.value).to.equal("last")
		end)

		it("includes helper logs in a success payload", function()
			local dependencies, state = makeRunDependencies()
			state.loaded = function(rojoExec)
				rojoExec.print("one")
				rojoExec.warn("two")
			end
			local payload = runWith(dependencies)
			expect(#payload.logs).to.equal(2)
			expect(payload.logs[1].level).to.equal("print")
			expect(payload.logs[1].message).to.equal("one")
			expect(payload.logs[2].level).to.equal("warn")
			expect(payload.logs[2].message).to.equal("two")
		end)

		it("classifies require errors as compile failures", function()
			local dependencies, state = makeRunDependencies()
			dependencies.requireTemporaryModule = function()
				error("Requested module experienced an error while loading")
			end
			local payload = runWith(dependencies, "local =", "compile.lua")
			expect(payload.outcome).to.equal("compileFailure")
			expect(payload.error:find("Requested module experienced", 1, true)).to.be.ok()
			expect(payload.error:find("__RojoExec_", 1, true)).to.be.ok()
			expect(state.invocationCount).to.equal(0)
		end)

		it("captures runtime failures and tracebacks", function()
			local dependencies, state = makeRunDependencies()
			state.loaded = function()
				error("intentional runtime failure")
			end
			local payload = runWith(dependencies)
			expect(payload.outcome).to.equal("runtimeFailure")
			expect(payload.error:find("intentional runtime failure", 1, true)).to.be.ok()
			expect(payload.traceback:find("traceback:", 1, true)).to.be.ok()
		end)

		it("starts and finishes one recording", function()
			local dependencies, state = makeRunDependencies()
			local payload = runWith(dependencies)
			expect(payload.outcome).to.equal("success")
			expect(state.finishCount).to.equal(1)
		end)

		it("commits the recording after a partial runtime failure", function()
			local dependencies, state = makeRunDependencies()
			state.loaded = function()
				error("after mutation")
			end
			local payload = runWith(dependencies)
			expect(payload.outcome).to.equal("runtimeFailure")
			expect(state.finishCount).to.equal(1)
		end)

		it("cleans up exactly once", function()
			local dependencies, state = makeRunDependencies()
			local controller = helpers.makeExecutionController(dependencies)
			helpers.runJob(claimedJob(), dependencies, controller)
			controller.cleanup()
			expect(state.cleanupCount).to.equal(1)
		end)

		it("does not invoke when recording cannot start", function()
			local dependencies, state = makeRunDependencies()
			state.recording = nil
			local payload = runWith(dependencies)
			expect(payload.outcome).to.equal("rejected")
			expect(state.invocationCount).to.equal(0)
			expect(state.finishCount).to.equal(0)
		end)

		it("rejects an edit-to-play race before invocation", function()
			local dependencies, state = makeRunDependencies()
			dependencies.isEdit = function()
				state.editCheckCount += 1
				return state.editCheckCount < 3
			end
			local payload = runWith(dependencies)
			expect(payload.outcome).to.equal("rejected")
			expect(state.invocationCount).to.equal(0)
			expect(state.finishCount).to.equal(1)
		end)

		it("cleans up after temporary module configuration fails", function()
			local dependencies, state = makeRunDependencies()
			dependencies.configureTemporaryModule = function()
				error("Source assignment was rejected")
			end

			local payload = runWith(dependencies)

			expect(payload.outcome).to.equal("runtimeFailure")
			expect(payload.error:find("Source assignment was rejected", 1, true)).to.be.ok()
			expect(state.invocationCount).to.equal(0)
			expect(state.finishCount).to.equal(0)
			expect(state.cleanupCount).to.equal(1)
		end)

		it("returns a timeout payload and cancels the worker", function()
			local dependencies, state = makeRunDependencies()
			local releaseWorker = Instance.new("BindableEvent")
			local resolveInvocation
			local invocationPromise = Promise.new(function(resolve)
				resolveInvocation = resolve
			end)
			local resolveTimeout
			state.loaded = function()
				resolveInvocation()
				releaseWorker.Event:Wait()
			end
			dependencies.spawn = task.spawn
			dependencies.cancel = task.cancel
			dependencies.delay = function()
				return Promise.new(function(resolve)
					resolveTimeout = resolve
				end)
			end

			local executionPromise = helpers.startExecution(claimedJob(), dependencies, 0.01)
			expect(invocationPromise:await()).to.equal(true)
			resolveTimeout()
			local success, payload = executionPromise:await()
			releaseWorker:Destroy()

			expect(success).to.equal(true)
			expect(payload.outcome).to.equal("timeout")
			expect(state.finishCount).to.equal(1)
			expect(state.cleanupCount).to.equal(1)
		end)
	end)

	describe("Exec poller", function()
		it("classifies edit, play, run, and unknown modes", function()
			expect(helpers.classifyStudioMode(true, false, false)).to.equal("edit")
			expect(helpers.classifyStudioMode(false, false, true)).to.equal("play")
			expect(helpers.classifyStudioMode(false, true, true)).to.equal("run")
			expect(helpers.classifyStudioMode(false, false, false)).to.equal("unknown")
		end)

		it("includes Studio mode in claim and completion calls", function()
			local claimMode
			local completionMode
			local poller = Exec.new({
				apiContext = {
					claimNextExecJob = function(_, mode)
						claimMode = mode
						return Promise.resolve(claimedJob())
					end,
					completeExecJob = function(_, _jobId, _payload, mode)
						completionMode = mode
						return Promise.resolve({ status = "accepted" })
					end,
				},
				dependencies = {
					studioMode = function()
						return "edit"
					end,
					execute = function()
						return Promise.resolve({ outcome = "success", logs = {} })
					end,
				},
			})

			poller:start()
			expect(claimMode).to.equal("edit")
			expect(completionMode).to.equal("edit")
			poller:stop()
		end)

		it("starts only once", function()
			local scheduler = makeScheduler()
			local claimCount = 0
			local poller = Exec.new({
				apiContext = {
					claimNextExecJob = function()
						claimCount += 1
						return Promise.resolve(nil)
					end,
				},
				dependencies = {
					delay = scheduler.delay,
					isEdit = function()
						return true
					end,
				},
			})
			poller:start()
			poller:start()
			expect(claimCount).to.equal(1)
			poller:stop()
		end)

		it("treats an empty claim as idle polling", function()
			local scheduler = makeScheduler()
			local executeCount = 0
			local poller = Exec.new({
				apiContext = {
					claimNextExecJob = function()
						return Promise.resolve(nil)
					end,
				},
				dependencies = {
					delay = scheduler.delay,
					isEdit = function()
						return true
					end,
					execute = function()
						executeCount += 1
						return Promise.resolve()
					end,
				},
			})
			poller:start()
			expect(executeCount).to.equal(0)
			expect(scheduler.count()).to.equal(1)
			poller:stop()
		end)

		it("claims and executes one job exactly once", function()
			local scheduler = makeScheduler()
			local executeCount = 0
			local completeCount = 0
			local api = {
				claimNextExecJob = function()
					return Promise.resolve(claimedJob())
				end,
				completeExecJob = function()
					completeCount += 1
					return Promise.resolve({ status = "accepted" })
				end,
			}
			local poller = Exec.new({
				apiContext = api,
				dependencies = {
					delay = scheduler.delay,
					isEdit = function()
						return true
					end,
					execute = function()
						executeCount += 1
						return Promise.resolve({ outcome = "success", result = { kind = "nil" }, logs = {} })
					end,
				},
			})
			poller:start()
			expect(executeCount).to.equal(1)
			expect(completeCount).to.equal(1)
			poller:stop()
		end)

		it("does not claim again while execution is pending", function()
			local scheduler = makeScheduler()
			local claimCount = 0
			local resolveExecution
			local poller = Exec.new({
				apiContext = {
					claimNextExecJob = function()
						claimCount += 1
						return Promise.resolve(claimedJob())
					end,
				},
				dependencies = {
					delay = scheduler.delay,
					isEdit = function()
						return true
					end,
					execute = function()
						return Promise.new(function(resolve)
							resolveExecution = resolve
						end)
					end,
				},
			})
			poller:start()
			poller:__poll(poller.__generation)
			expect(claimCount).to.equal(1)
			poller:stop()
			resolveExecution({ outcome = "success", logs = {} })
		end)

		it("stops scheduled polling", function()
			local scheduler = makeScheduler()
			local claimCount = 0
			local poller = Exec.new({
				apiContext = {
					claimNextExecJob = function()
						claimCount += 1
						return Promise.resolve(nil)
					end,
				},
				dependencies = {
					delay = scheduler.delay,
					isEdit = function()
						return true
					end,
				},
			})
			poller:start()
			poller:stop()
			scheduler.step()
			expect(claimCount).to.equal(1)
		end)

		it("ignores a late claim after stop", function()
			local resolveClaim
			local executeCount = 0
			local poller = Exec.new({
				apiContext = {
					claimNextExecJob = function()
						return Promise.new(function(resolve)
							resolveClaim = resolve
						end)
					end,
				},
				dependencies = {
					isEdit = function()
						return true
					end,
					execute = function()
						executeCount += 1
						return Promise.resolve()
					end,
				},
			})
			poller:start()
			poller:stop()
			resolveClaim(claimedJob())
			expect(executeCount).to.equal(0)
		end)

		it("cancels active execution when the session stops", function()
			local cancellationCount = 0
			local completionCount = 0
			local poller = Exec.new({
				apiContext = {
					claimNextExecJob = function()
						return Promise.resolve(claimedJob())
					end,
					completeExecJob = function()
						completionCount += 1
						return Promise.resolve({ status = "accepted" })
					end,
				},
				dependencies = {
					isEdit = function()
						return true
					end,
					execute = function()
						return Promise.new(function(_resolve, _reject, onCancel)
							onCancel(function()
								cancellationCount += 1
							end)
						end)
					end,
				},
			})
			poller:start()
			poller:stop()
			expect(cancellationCount).to.equal(1)
			expect(completionCount).to.equal(0)
		end)

		it("does not claim in play mode and resumes in edit mode", function()
			local scheduler = makeScheduler()
			local edit = false
			local claimCount = 0
			local poller = Exec.new({
				apiContext = {
					claimNextExecJob = function()
						claimCount += 1
						return Promise.resolve(nil)
					end,
				},
				dependencies = {
					delay = scheduler.delay,
					isEdit = function()
						return edit
					end,
				},
			})
			poller:start()
			expect(claimCount).to.equal(0)
			edit = true
			scheduler.step()
			expect(claimCount).to.equal(1)
			poller:stop()
		end)

		it("retries completion without re-executing", function()
			local scheduler = makeScheduler()
			local executeCount = 0
			local completeCount = 0
			local poller = Exec.new({
				apiContext = {
					claimNextExecJob = function()
						return Promise.resolve(claimedJob())
					end,
					completeExecJob = function()
						completeCount += 1
						if completeCount < 3 then
							return Promise.reject("transport failure")
						end
						return Promise.resolve({ status = "accepted" })
					end,
				},
				dependencies = {
					delay = scheduler.delay,
					isEdit = function()
						return true
					end,
					execute = function()
						executeCount += 1
						return Promise.resolve({ outcome = "success", logs = {} })
					end,
				},
			})
			poller:start()
			scheduler.step()
			scheduler.step()
			expect(completeCount).to.equal(3)
			expect(executeCount).to.equal(1)
			poller:stop()
		end)

		it("treats completion conflict as terminal without re-executing", function()
			local scheduler = makeScheduler()
			local executeCount = 0
			local poller = Exec.new({
				apiContext = {
					claimNextExecJob = function()
						return Promise.resolve(claimedJob())
					end,
					completeExecJob = function()
						return Promise.resolve({ status = "conflict" })
					end,
				},
				dependencies = {
					delay = scheduler.delay,
					isEdit = function()
						return true
					end,
					execute = function()
						executeCount += 1
						return Promise.resolve({ outcome = "success", logs = {} })
					end,
				},
			})
			poller:start()
			expect(executeCount).to.equal(1)
			poller:stop()
		end)

		it("routes malformed claim failures to the session error callback", function()
			local capturedError = nil
			local poller = Exec.new({
				apiContext = {
					claimNextExecJob = function()
						return Promise.reject("Prism exec protocol error: malformed claimed-job response")
					end,
				},
				dependencies = {
					isEdit = function()
						return true
					end,
				},
				onError = function(errorValue)
					capturedError = errorValue
				end,
			})
			poller:start()
			expect(tostring(capturedError):find("protocol error", 1, true)).to.be.ok()
			poller:stop()
		end)

		it("validates the exact claimed-job wire shape", function()
			local valid = Types.ApiExecClaimResponse(claimedJob())
			local malformed = Types.ApiExecClaimResponse({
				jobId = claimedJob().jobId,
				scriptName = "test.lua",
				state = "claimed",
			})
			expect(valid).to.equal(true)
			expect(malformed).to.equal(false)
		end)
	end)
end
