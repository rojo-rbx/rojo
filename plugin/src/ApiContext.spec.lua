return function()
	local ApiContext = require(script.Parent.ApiContext)
	local Types = require(script.Parent.Types)
	local helpers = ApiContext._test

	describe("automation plugin session identity", function()
		it("retains one generated ID for an active session", function()
			local context = ApiContext.new("http://127.0.0.1:34872")
			local generateCount = 0
			local id = helpers.beginPluginSession(context, "server-one", function()
				generateCount += 1
				return "00000000-0000-4000-8000-000000000001"
			end)

			expect(id).to.equal("00000000-0000-4000-8000-000000000001")
			expect(context:getPluginSessionId()).to.equal(id)
			expect(generateCount).to.equal(1)
		end)

		it("creates a new ID for a new connection lifecycle", function()
			local context = ApiContext.new("http://127.0.0.1:34872")
			local nextId = 0
			local function generate()
				nextId += 1
				return ("00000000-0000-4000-8000-%012d"):format(nextId)
			end
			local first = helpers.beginPluginSession(context, "server-one", generate)
			context:disconnect()
			local second = helpers.beginPluginSession(context, "server-one", generate)

			expect(first).never.to.equal(second)
			expect(context:getPluginSessionId()).to.equal(second)
		end)

		it("adds identity and mode to claims and completions", function()
			local id = "00000000-0000-4000-8000-000000000001"
			local url = helpers.buildExecClaimUrl("http://127.0.0.1:34872", id, "edit")
			local payload = helpers.withExecSession({ outcome = "success" }, id, "run")

			expect(url:find("pluginSessionId=" .. id, 1, true)).to.be.ok()
			expect(url:find("studioMode=edit", 1, true)).to.be.ok()
			expect(payload.pluginSessionId).to.equal(id)
			expect(payload.studioMode).to.equal("run")
			expect(payload.outcome).to.equal("success")

			local automationUrl = helpers.buildAutomationClaimUrl("http://127.0.0.1:34872", id, "edit")
			expect(automationUrl:find("/api/automation/jobs/next", 1, true)).to.be.ok()
			expect(automationUrl:find("pluginSessionId=" .. id, 1, true)).to.be.ok()
		end)

		it("validates all supported Studio modes", function()
			for _, mode in { "edit", "play", "run", "unknown" } do
				expect(Types.StudioMode(mode)).to.equal(true)
			end
			expect(Types.StudioMode("server")).to.equal(false)
		end)
	end)
end
