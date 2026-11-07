return function()
	local Promise = require(script.Parent.Parent.Packages.Promise)
	local ServeSession = require(script.Parent.ServeSession)
	local PatchSet = require(script.Parent.PatchSet)

	local session
	local api

	beforeEach(function()
		api = {
			disconnect = function() end,
		}
		session = ServeSession.new({ apiContext = api, twoWaySync = false })
	end)

	afterEach(function()
		session:stop()
	end)

	describe("sync fallback replacement", function()
		-- The shape of the model returned by the serialize endpoint.
		local function model(ids)
			local root = Instance.new("Folder")
			for _, id in ids do
				local pointer = Instance.new("ObjectValue")
				pointer.Name = id
				pointer.Value = Instance.new("Folder")
				pointer.Parent = root
			end
			return { root }
		end

		it("leaves out ids the server reports as missing", function()
			local container = Instance.new("Folder")
			local present = Instance.new("Folder")
			present.Parent = container
			local missing = Instance.new("Folder")
			missing.Parent = container
			session.__instanceMap:insert("PRESENT", present)
			session.__instanceMap:insert("MISSING", missing)

			local refPatchIds
			api.serialize = function()
				return Promise.resolve({ modelContents = buffer.create(0), missingIds = { "MISSING" } })
			end
			api.refPatch = function(_, ids)
				refPatchIds = ids
				return Promise.resolve({ patch = PatchSet.newEmpty() })
			end
			session.__deserializeInstances = function()
				return model({ "PRESENT" })
			end

			local ok, unapplied = session:__replaceInstances({ "PRESENT", "MISSING" })
			expect(ok).to.equal(true)
			expect(PatchSet.isEmpty(unapplied)).to.equal(true)
			expect(#refPatchIds).to.equal(1)
			expect(refPatchIds[1]).to.equal("PRESENT")
			expect(session.__instanceMap.fromIds["PRESENT"]).never.to.equal(present)
			expect(session.__instanceMap.fromIds["MISSING"]).to.equal(missing)
			expect(missing.Parent).to.equal(container)
			container:Destroy()
		end)

		it("succeeds without further requests when every id is missing", function()
			local refPatchCalls = 0
			api.serialize = function()
				return Promise.resolve({ modelContents = buffer.create(0), missingIds = { "GONE" } })
			end
			api.refPatch = function()
				refPatchCalls += 1
				return Promise.resolve({ patch = PatchSet.newEmpty() })
			end
			session.__deserializeInstances = function()
				error("nothing should be deserialized")
			end

			local ok, unapplied = session:__replaceInstances({ "GONE" })
			expect(ok).to.equal(true)
			expect(PatchSet.isEmpty(unapplied)).to.equal(true)
			expect(refPatchCalls).to.equal(0)
		end)

		it("still fails when the server returns fewer instances without reporting them", function()
			api.serialize = function()
				return Promise.resolve({ modelContents = buffer.create(0) })
			end
			api.refPatch = function()
				return Promise.resolve({ patch = PatchSet.newEmpty() })
			end
			session.__deserializeInstances = function()
				return model({ "A" })
			end

			local ok = session:__replaceInstances({ "A", "B" })
			expect(ok).to.equal(false)
		end)
	end)
end
