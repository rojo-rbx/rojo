return function()
	local ChangeBatcher = require(script.Parent.ChangeBatcher)

	local noop = function() end

	beforeEach(function(context)
		local mockInstanceMap = {
			pausedBatchInstances = {},
			fromInstances = {},

			addInstance = function(self, instance)
				self.fromInstances[instance] = 0
			end,
		}

		local changeBatcher = ChangeBatcher.new(mockInstanceMap, noop)

		changeBatcher.__heartbeatConnection:Disconnect()

		context.mockInstanceMap = mockInstanceMap
		context.changeBatcher = changeBatcher
	end)

	describe("new", function()
		it("should create a new ChangeBatcher", function(context)
			local changeBatcher = context.changeBatcher

			expect(changeBatcher.__pendingChanges).to.be.a("table")
			expect(next(changeBatcher.__pendingChanges)).to.equal(nil)
			expect(changeBatcher.__instanceMap).to.equal(context.mockInstanceMap)
			expect(typeof(changeBatcher.__heartbeatConnection)).to.equal("RBXScriptConnection")
		end)
	end)

	describe("add", function()
		it("should add property changes to be considered for the current batch", function(context)
			local changeBatcher = context.changeBatcher
			local part = Instance.new("Part")

			changeBatcher:add(part, "Name")

			local properties = changeBatcher.__pendingChanges[part]

			expect(properties).to.be.a("table")
			expect(properties.Name).to.be.ok()

			changeBatcher:add(part, "Position")
			expect(properties.Position).to.be.ok()

			changeBatcher.__heartbeatConnection:Disconnect()
		end)
	end)

	describe("__cycle", function()
		it("should unpause instances that were paused for the current cycle in the next cycle", function(context)
			local changeBatcher = context.changeBatcher
			local bindableEvent = Instance.new("BindableEvent")
			local part = Instance.new("Part")

			changeBatcher.__instanceMap.pausedBatchInstances[part] = true

			changeBatcher:__cycle(0)
			changeBatcher:__cycle(0)

			expect(changeBatcher.__instanceMap.pausedBatchInstances[part]).to.equal(nil)
		end)
	end)

	describe("__flush", function()
		it("should return nil when there are no change to process", function(context)
			local changeBatcher = context.changeBatcher
			expect(changeBatcher:__flush()).to.equal(nil)
		end)

		it("should return a patch when there are changes to process and the patch will be non-empty", function(context)
			local changeBatcher = context.changeBatcher
			local part = Instance.new("Part")

			changeBatcher.__instanceMap:addInstance(part)
			changeBatcher.__pendingChanges[part] = {
				Position = true,
				Name = true,
			}

			local patch = changeBatcher:__flush()

			expect(patch).to.be.a("table")
			expect(patch.updated).to.be.a("table")
			expect(patch.removed).to.be.a("table")
			expect(patch.added).to.be.a("table")
		end)

		it("should encode changed properties as updates in the patch", function(context)
			local changeBatcher = context.changeBatcher
			local part = Instance.new("Part")
			local model = Instance.new("Model")
			local changes = {
				Name = true,
				Parent = true,
			}

			changeBatcher.__instanceMap:addInstance(part)
			changeBatcher.__instanceMap:addInstance(model)
			changeBatcher.__pendingChanges[part] = changes
			changeBatcher.__pendingChanges[model] = changes

			local patch = changeBatcher:__flush()

			expect(#patch.updated).to.equal(2)

			for _, update in ipairs(patch.updated) do
				for propertyName in pairs(update.changedProperties) do
					-- We don't really care what the encoded values look like exactly,
					-- just that they're there.
					expect(changes[propertyName]).to.be.ok()
				end
			end
		end)

		it("should return nil when there are changes to process and the patch will be empty", function(context)
			local changeBatcher = context.changeBatcher
			local part = Instance.new("Part")

			changeBatcher.__instanceMap:addInstance(part)
			changeBatcher.__pendingChanges[part] = {
				NonExistentProperty = true,
			}

			expect(changeBatcher:__flush()).to.equal(nil)
		end)
	end)
end
