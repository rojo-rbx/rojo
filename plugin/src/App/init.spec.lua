return function()
	local App = require(script.Parent)
	local ServeSession = require(script.Parent.Parent.ServeSession)
	local ApiContext = require(script.Parent.Parent.ApiContext)

	-- The module returns a wrapper function that renders the App component.
	-- The App class itself isn't exported, so we pull it off the element it
	-- creates in order to call its methods directly.
	local AppComponent = App({}).component

	-- A stubbed ServeSession that records nothing and does nothing, so that
	-- startSession can run without touching the network.
	local function makeServeSessionStub()
		return {
			setUpdateLoadingTextCallback = function() end,
			hookPrecommit = function()
				return function() end
			end,
			hookPostcommit = function()
				return function() end
			end,
			onStatusChanged = function() end,
			setConfirmCallback = function() end,
			start = function() end,
		}
	end

	-- A minimal `self` with just the methods startSession touches, so we can
	-- invoke App.startSession without mounting the whole component.
	local function makeFakeSelf()
		return {
			serveSession = nil,
			knownProjects = {},
			-- claimSyncLock returns true in solo (no Team Create), which is the
			-- "quick session" case the double-click bug happens in.
			claimSyncLock = function()
				return true
			end,
			getHostAndPort = function()
				return "localhost", "34872"
			end,
			isAutoConnectPlaytestServerAvailable = function()
				return false
			end,
			setState = function() end,
			addNotification = function() end,
			setPriorSyncInfo = function() end,
			setRunningConnectionInfo = function() end,
			clearRunningConnectionInfo = function() end,
			releaseSyncLock = function() end,
		}
	end

	describe("startSession", function()
		local realServeSessionNew = ServeSession.new
		local realApiContextNew = ApiContext.new
		local created

		beforeEach(function()
			created = 0
			ServeSession.new = function()
				created += 1
				return makeServeSessionStub()
			end
			-- Stub ApiContext so construction stays offline and deterministic.
			ApiContext.new = function()
				return {}
			end
		end)

		afterEach(function()
			ServeSession.new = realServeSessionNew
			ApiContext.new = realApiContextNew
		end)

		it("creates a session on the first call", function()
			local self = makeFakeSelf()
			AppComponent.startSession(self)
			expect(created).to.equal(1)
		end)

		it("does not start a second session when called again while active", function()
			local self = makeFakeSelf()
			-- Two calls in a row mimic a double-click firing startSession twice
			-- before the first session finishes. Without the re-entrancy guard
			-- this creates two ServeSessions and leaks the first.
			AppComponent.startSession(self)
			AppComponent.startSession(self)
			expect(created).to.equal(1)
		end)

		it("allows a new session after the previous one is cleared", function()
			local self = makeFakeSelf()
			AppComponent.startSession(self)
			-- Disconnecting clears serveSession; a later connect should work.
			self.serveSession = nil
			AppComponent.startSession(self)
			expect(created).to.equal(2)
		end)
	end)
end
