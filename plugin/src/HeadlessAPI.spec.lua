return function()
	local HeadlessAPI = require(script.Parent.HeadlessAPI)
	local Settings = require(script.Parent.Settings)

	-- The API only starts and stops sessions, so a stub app is enough here.
	-- onStartSession decides how the session this stub was asked for settles.
	local function newStubApp()
		local stub = { settleCount = 0 }

		function stub:startSession(host, port, onSettled)
			self.startedWith = { host = host, port = port }
			self.settle = function(success, message)
				self.settleCount += 1
				onSettled(success, message)
			end

			if self.onStartSession then
				self.onStartSession()
			end
		end

		function stub:endSession() end

		return stub
	end

	-- A caller's source is read off the traceback, and the test runner is itself
	-- a plugin, so it is stubbed to name the caller each test is about.
	local function newApiAsSource(source: string)
		local api, readOnlyApi = HeadlessAPI.new(newStubApp())

		function api:_getCallerSource()
			return source
		end

		return api, readOnlyApi
	end

	it("should tell the caller when a session connects", function()
		local app = newStubApp()
		local api = HeadlessAPI.new(app)

		app.onStartSession = function()
			app.settle(true)
		end

		local success, message = api:ConnectAsync("localhost", "34872")

		expect(success).to.equal(true)
		expect(message).to.equal(nil)
		expect(app.startedWith.host).to.equal("localhost")
		expect(app.startedWith.port).to.equal("34872")
	end)

	it("should tell the caller when a session fails before it starts", function()
		local app = newStubApp()
		local api = HeadlessAPI.new(app)

		-- A held sync lock settles inside startSession, before the caller yields.
		app.onStartSession = function()
			app.settle(false, "Could not sync because user 'someone' is already syncing")
		end

		local success, message = api:ConnectAsync()

		expect(success).to.equal(false)
		expect(message).to.equal("Could not sync because user 'someone' is already syncing")
	end)

	it("should wait for a session that settles later", function()
		local app = newStubApp()
		local api = HeadlessAPI.new(app)

		app.onStartSession = function()
			task.delay(0.05, function()
				app.settle(false, "Connection refused")
			end)
		end

		local success, message = api:ConnectAsync()

		expect(success).to.equal(false)
		expect(message).to.equal("Connection refused")
	end)

	it("should answer a caller once, even if its session settles again", function()
		local app = newStubApp()
		local api = HeadlessAPI.new(app)

		app.onStartSession = function()
			app.settle(true)
		end

		expect(api:ConnectAsync()).to.equal(true)
		expect(app.settleCount).to.equal(1)

		-- The session reporting again, as it does when it later disconnects, must
		-- not reach a caller that already has its answer.
		app.settle(false, "Disconnected from session")

		expect(app.settleCount).to.equal(2)
		expect(api:ConnectAsync()).to.equal(true)
	end)

	it("should refuse callers that have not been granted access", function()
		local _, readOnlyApi = newApiAsSource("user_Impostor.rbxmx")

		expect(function()
			return readOnlyApi.ConnectAsync
		end).to.throw()

		expect(readOnlyApi.Version).to.be.ok()
	end)

	it("should store permissions without putting a source in a table key", function()
		local priorPermissions = Settings:get("apiPermissions")
		local api = newApiAsSource("user_Companion.rbxmx")

		api:_setPermissions("user_Companion.rbxmx", "Companion", { "ConnectAsync" })

		local stored = Settings:get("apiPermissions")
		expect(#stored).to.equal(1)
		expect(stored[1].source).to.equal("user_Companion.rbxmx")
		expect(stored[1].apis[1]).to.equal("ConnectAsync")

		local _, readOnlyApi = newApiAsSource("user_Companion.rbxmx")
		expect(readOnlyApi.ConnectAsync).to.be.a("function")

		api:_removePermissions("user_Companion.rbxmx", "Companion")
		expect(#Settings:get("apiPermissions")).to.equal(0)

		Settings:set("apiPermissions", priorPermissions)
	end)
end
