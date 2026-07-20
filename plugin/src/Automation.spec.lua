return function()
	local Packages = script.Parent.Parent.Packages
	local Promise = require(Packages.Promise)
	local Automation = require(script.Parent.Automation)

	local function scheduler()
		local pending = {}
		return {
			delay = function()
				return Promise.new(function(resolve)
					table.insert(pending, resolve)
				end)
			end,
			step = function()
				local resolve = table.remove(pending, 1)
				assert(resolve ~= nil, "no pending timer")
				resolve()
			end,
			count = function()
				return #pending
			end,
		}
	end

	local function fakeReferences()
		return { clear = function() end }
	end

	describe("typed automation poller", function()
		it("starts once, idles on empty claims, and stops", function()
			local timers = scheduler()
			local claims = 0
			local api = {
				getPluginSessionId = function()
					return "plugin-session"
				end,
				claimNextAutomationJob = function()
					claims += 1
					return Promise.resolve(nil)
				end,
			}
			local poller = Automation.new({
				apiContext = api,
				dependencies = {
					delay = timers.delay,
					studioMode = function()
						return "edit"
					end,
					makeReferences = fakeReferences,
				},
			})
			poller:start()
			poller:start()
			expect(timers.count()).to.equal(1)
			timers.step()
			expect(claims).to.equal(1)
			poller:stop()
		end)

		it("does not claim outside edit mode", function()
			local timers = scheduler()
			local claims = 0
			local poller = Automation.new({
				apiContext = {
					getPluginSessionId = function()
						return "session"
					end,
					claimNextAutomationJob = function()
						claims += 1
						return Promise.resolve(nil)
					end,
				},
				dependencies = {
					delay = timers.delay,
					studioMode = function()
						return "play"
					end,
					makeReferences = fakeReferences,
				},
			})
			poller:start()
			timers.step()
			expect(claims).to.equal(0)
			poller:stop()
		end)

		it("dispatches and completes one claimed inspect job", function()
			local timers = scheduler()
			local dispatches = 0
			local completions = 0
			local api = {
				getPluginSessionId = function()
					return "session"
				end,
				claimNextAutomationJob = function()
					return Promise.resolve({ jobId = "job", request = { kind = "inspect" } })
				end,
				completeAutomationJob = function(_, _, payload)
					completions += 1
					expect(payload.outcome).to.equal("success")
					return Promise.resolve({ status = "accepted" })
				end,
			}
			local poller = Automation.new({
				apiContext = api,
				dependencies = {
					delay = timers.delay,
					studioMode = function()
						return "edit"
					end,
					makeReferences = fakeReferences,
					dispatch = function()
						dispatches += 1
						return { outcome = "success", result = { kind = "inspect" } }
					end,
				},
			})
			poller:start()
			timers.step()
			expect(dispatches).to.equal(1)
			expect(completions).to.equal(1)
			poller:stop()
		end)

		it("turns unknown kinds into typed failures", function()
			local payload = Automation._test.dispatch({ request = { kind = "unknown" } }, {})
			expect(payload.outcome).to.equal("failure")
		end)
	end)
end
