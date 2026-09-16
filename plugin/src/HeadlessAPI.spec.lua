return function()
	local HeadlessAPI = require(script.Parent.HeadlessAPI)
	local Settings = require(script.Parent.Settings)

	-- The API only reaches back into the app to start and stop sessions, so a
	-- stub is enough to observe what a caller is told about the attempt.
	local function newStubApp()
		local stub = {
			serveSession = nil,
			startedWith = nil,
		}

		function stub:startSession(host, port)
			stub.startedWith = { host = host, port = port }

			if stub.onStartSession then
				stub.onStartSession()
			end
		end

		function stub:endSession() end

		return stub
	end

	it("should tell the caller when a session connects", function()
		local app = newStubApp()
		local api = HeadlessAPI.new(app)

		app.onStartSession = function()
			api:_settleConnectAttempt(true)
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

		-- Failures such as a held sync lock settle inside startSession, before
		-- the calling thread ever has a chance to yield.
		app.onStartSession = function()
			api:_settleConnectAttempt(false, "Could not sync because user 'someone' is already syncing")
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
				api:_settleConnectAttempt(false, "Connection refused")
			end)
		end

		local success, message = api:ConnectAsync()

		expect(success).to.equal(false)
		expect(message).to.equal("Connection refused")
	end)

	it("should only settle the attempts that are still waiting", function()
		local app = newStubApp()
		local api = HeadlessAPI.new(app)

		app.onStartSession = function()
			api:_settleConnectAttempt(true)
		end

		expect(api:ConnectAsync()).to.equal(true)

		-- A later disconnect has nobody to report to and must not resume the
		-- thread that already got its answer.
		api:_settleConnectAttempt(false, "Disconnected from session")

		expect(api:ConnectAsync()).to.equal(true)
	end)

	-- Which source a call comes from is read off the traceback, and the test
	-- runner is itself a plugin, so the source is stubbed to describe the caller
	-- the test is about.
	local function newApiAsSource(source: string)
		local api, readOnlyApi = HeadlessAPI.new(newStubApp())

		function api:_getCallerSource()
			return source
		end

		return api, readOnlyApi
	end

	it("should refuse callers that have not been granted access", function()
		local _, readOnlyApi = newApiAsSource("user_Impostor.rbxmx")

		expect(function()
			return readOnlyApi.ConnectAsync
		end).to.throw()

		-- Reading the version is how a caller checks compatibility, so it never
		-- needs a grant.
		expect(readOnlyApi.Version).to.be.ok()
	end)

	it("should store permissions without putting a source in a table key", function()
		local api = newApiAsSource("user_Companion.rbxmx")
		api:_setPermissions("user_Companion.rbxmx", "Companion", { "ConnectAsync" })

		-- plugin:SetSetting rewrites '.' in keys, which would make the stored
		-- grant unmatchable the next time Studio opens.
		local stored = Settings:get("apiPermissions")
		expect(#stored).to.equal(1)
		expect(stored[1].source).to.equal("user_Companion.rbxmx")
		expect(stored[1].apis[1]).to.equal("ConnectAsync")

		-- A fresh API reads that back, which is what a new Studio session does.
		local _, readOnlyApi = newApiAsSource("user_Companion.rbxmx")
		expect(readOnlyApi.ConnectAsync).to.be.a("function")

		api:_removePermissions("user_Companion.rbxmx", "Companion")
		expect(#Settings:get("apiPermissions")).to.equal(0)
	end)
end
