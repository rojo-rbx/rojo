return function()
	local PatchSet = require(script.Parent.Parent.PatchSet)
	local InstanceMap = require(script.Parent.Parent.InstanceMap)

	local createPatchSet = require(script.Parent.createPatchSet)

	it("should return a patch", function()
		local patch = createPatchSet(InstanceMap.new(), {})

		assert(PatchSet.validate(patch))
	end)

	it("should contain updates for every instance with property changes", function()
		local instanceMap = InstanceMap.new()

		local part1 = Instance.new("Part")
		instanceMap:insert("PART_1", part1)

		local part2 = Instance.new("Part")
		instanceMap:insert("PART_2", part2)

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

		local patch = createPatchSet(instanceMap, changes)

		expect(#patch.updated).to.equal(2)
	end)

	it("should not contain any updates for removed instances", function()
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

		local patch = createPatchSet(instanceMap, changes)

		expect(#patch.removed).to.equal(1)
		expect(#patch.updated).to.equal(0)
	end)

	it("should remove instances from the property change table", function()
		local instanceMap = InstanceMap.new()

		local part1 = Instance.new("Part")
		instanceMap:insert("PART_1", part1)

		local changes = {
			[part1] = {},
		}

		createPatchSet(instanceMap, changes)

		expect(next(changes)).to.equal(nil)
	end)
end
