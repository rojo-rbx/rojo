return function()
	local Packages = script.Parent.Parent.Packages
	local Promise = require(Packages.Promise)
	local ServeSession = require(script.Parent.ServeSession)
	local AutomationHeartbeat = ServeSession._test.AutomationHeartbeat

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
				assert(resolve ~= nil, "no heartbeat delay scheduled")
				resolve()
			end,
			count = function()
				return #queue
			end,
		}
	end

	describe("automation heartbeat", function()
		it("signals readiness once after registration", function()
			local readyCount = 0
			local heartbeat = AutomationHeartbeat.new({
				apiContext = {
					updateAutomationStatus = function()
						return Promise.resolve({ registration = "registered" })
					end,
				},
				delay = function()
					return Promise.new(function() end)
				end,
				getStudioMode = function()
					return "edit"
				end,
				onReady = function()
					readyCount += 1
				end,
				onError = function(errorValue)
					error(errorValue)
				end,
			})
			heartbeat:start()
			expect(readyCount).to.equal(1)
			heartbeat:stop()
		end)

		it("starts once and reports the current mode", function()
			local scheduler = makeScheduler()
			local updateCount = 0
			local reportedMode
			local heartbeat = AutomationHeartbeat.new({
				apiContext = {
					updateAutomationStatus = function(_, mode)
						updateCount += 1
						reportedMode = mode
						return Promise.resolve({ registration = "registered" })
					end,
				},
				delay = scheduler.delay,
				getStudioMode = function()
					return "run"
				end,
				onError = function(errorValue)
					error(errorValue)
				end,
			})

			heartbeat:start()
			heartbeat:start()
			expect(updateCount).to.equal(1)
			expect(reportedMode).to.equal("run")
			expect(scheduler.count()).to.equal(1)
			heartbeat:stop()
		end)

		it("stops scheduling updates after disconnect", function()
			local scheduler = makeScheduler()
			local updateCount = 0
			local heartbeat = AutomationHeartbeat.new({
				apiContext = {
					updateAutomationStatus = function()
						updateCount += 1
						return Promise.resolve({ registration = "refreshed" })
					end,
				},
				delay = scheduler.delay,
				getStudioMode = function()
					return "edit"
				end,
				onError = function(errorValue)
					error(errorValue)
				end,
			})

			heartbeat:start()
			heartbeat:stop()
			scheduler.step()
			expect(updateCount).to.equal(1)
		end)

		it("ignores a late response from a stopped generation", function()
			local resolveHeartbeat
			local errorCount = 0
			local heartbeat = AutomationHeartbeat.new({
				apiContext = {
					updateAutomationStatus = function()
						return Promise.new(function(resolve)
							resolveHeartbeat = resolve
						end)
					end,
				},
				getStudioMode = function()
					return "edit"
				end,
				onError = function()
					errorCount += 1
				end,
			})

			heartbeat:start()
			heartbeat:stop()
			resolveHeartbeat({ registration = "conflict" })
			expect(errorCount).to.equal(0)
		end)

		it("surfaces duplicate-session conflicts clearly", function()
			local capturedError
			local heartbeat = AutomationHeartbeat.new({
				apiContext = {
					updateAutomationStatus = function()
						return Promise.resolve({ registration = "conflict" })
					end,
				},
				getStudioMode = function()
					return "edit"
				end,
				onError = function(errorValue)
					capturedError = errorValue
				end,
			})

			heartbeat:start()
			expect(capturedError:find("already registered", 1, true)).to.be.ok()
			heartbeat:stop()
		end)
	end)
end
