return function()
	local PatchSet = require(script.Parent.Parent.PatchSet)
	local InstanceMap = require(script.Parent.Parent.InstanceMap)

	local createChangePatch = require(script.Parent.createChangePatch)

	it("should return a patch", function(context)
		local patch = createChangePatch(InstanceMap.new(), {})

		assert(PatchSet.validate(patch))
	end)

	it("should encode updated properties in the patch", function()
		local instanceMap = InstanceMap.new()

		local part1 = Instance.new("Part")
		instanceMap:insert("PART_1", part1)

		local part2 = Instance.new("Part")
		instanceMap:insert("PART_2", part2)

		local expectedChanges = {
			[part1] = {
				Position = true,
				Size = true,
				Color = true,
			},
			[part2] = {
				CFrame = true,
				Velocity = true,
				Transparency = true,
			},
		}

		local changes = {
			[part1] = {
				Position = true,
				Size = true,
				Color = true,
			},
			[part2] = {
				CFrame = true,
				Velocity = true,
				Transparency = true,
			},
		}

		local patch = createChangePatch(instanceMap, changes)

		expect(#patch.updated).to.equal(2)

		for _, update in ipairs(patch.updated) do
			local instance = instanceMap.fromIds[update.id]
			local propertyChanges = expectedChanges[instance]

			expect(propertyChanges).to.be.ok()

			for propertyName in pairs(update.changedProperties) do
				expect(propertyChanges[propertyName]).to.be.ok()
			end
		end
	end)

	it("should not encode any updated properties when the instance was removed", function()
		local instanceMap = InstanceMap.new()

		local part1 = Instance.new("Part")
		instanceMap:insert("PART_1", part1)

		local changes = {
			[part1] = {
				Parent = true,
				Position = true,
				Size = true,
			},
		}

		local patch = createChangePatch(instanceMap, changes)

		expect(#patch.removed).to.equal(1)
		expect(#patch.updated).to.equal(0)
	end)

	it("should remove instances from the property update table", function()
		local instanceMap = InstanceMap.new()

		local part1 = Instance.new("Part")
		instanceMap:insert("PART_1", part1)

		local changes = {
			[part1] = {},
		}

		createChangePatch(instanceMap, changes)

		expect(next(changes)).to.equal(nil)
	end)
end
