return function()
	local reify = require(script.Parent.reify)

	local PatchSet = require(script.Parent.Parent.PatchSet)
	local InstanceMap = require(script.Parent.Parent.InstanceMap)

	local function isEmpty(table)
		return next(table) == nil, "Table was not empty"
	end

	local function size(dict)
		local len = 0

		for _ in pairs(dict) do
			len = len + 1
		end

		return len
	end

	it("should throw when given a bogus ID", function()
		expect(function()
			reify(InstanceMap.new(), {}, "Hi, mom!", game)
		end).to.throw()
	end)

	it("should return an error when given bogus class names", function()
		local virtualInstances = {
			ROOT = {
				ClassName = "Balogna",
				Name = "Food",
				Properties = {},
				Children = {},
			},
		}

		local instanceMap = InstanceMap.new()
		local unappliedPatch = reify(instanceMap, virtualInstances, "ROOT", nil)

		assert(instanceMap:size() == 0, "expected instanceMap to be empty")

		expect(size(unappliedPatch.added)).to.equal(1)
		expect(unappliedPatch.added["ROOT"]).to.equal(virtualInstances["ROOT"])

		assert(isEmpty(unappliedPatch.removed), "expected no removes")
		assert(isEmpty(unappliedPatch.updated), "expected no updates")
	end)

	it("should assign name and properties", function()
		local virtualInstances = {
			ROOT = {
				ClassName = "StringValue",
				Name = "Spaghetti",
				Properties = {
					Value = {
						String = "Hello, world!",
					},
				},
				Children = {},
			},
		}

		local instanceMap = InstanceMap.new()
		local unappliedPatch = reify(instanceMap, virtualInstances, "ROOT")

		assert(PatchSet.isEmpty(unappliedPatch), "expected remaining patch to be empty")

		local instance = instanceMap.fromIds["ROOT"]
		expect(instance.ClassName).to.equal("StringValue")
		expect(instance.Name).to.equal("Spaghetti")
		expect(instance.Value).to.equal("Hello, world!")

		expect(instanceMap:size()).to.equal(1)
	end)

	it("should construct children", function()
		local virtualInstances = {
			ROOT = {
				ClassName = "Folder",
				Name = "Parent",
				Properties = {},
				Children = { "CHILD" },
			},

			CHILD = {
				ClassName = "Folder",
				Name = "Child",
				Properties = {},
				Children = {},
			},
		}

		local instanceMap = InstanceMap.new()
		local unappliedPatch = reify(instanceMap, virtualInstances, "ROOT")

		assert(PatchSet.isEmpty(unappliedPatch), "expected remaining patch to be empty")

		local instance = instanceMap.fromIds["ROOT"]
		expect(instance.ClassName).to.equal("Folder")
		expect(instance.Name).to.equal("Parent")

		local child = instance.Child
		expect(child.ClassName).to.equal("Folder")

		expect(instanceMap:size()).to.equal(2)
	end)

	it("should still construct parents if children fail", function()
		local virtualInstances = {
			ROOT = {
				ClassName = "Folder",
				Name = "Parent",
				Properties = {},
				Children = { "CHILD" },
			},

			CHILD = {
				ClassName = "this ain't an Instance",
				Name = "Child",
				Properties = {},
				Children = {},
			},
		}

		local instanceMap = InstanceMap.new()
		local unappliedPatch = reify(instanceMap, virtualInstances, "ROOT")

		expect(size(unappliedPatch.added)).to.equal(1)
		expect(unappliedPatch.added["CHILD"]).to.equal(virtualInstances["CHILD"])
		assert(isEmpty(unappliedPatch.updated), "expected no updates")
		assert(isEmpty(unappliedPatch.removed), "expected no removes")

		local instance = instanceMap.fromIds["ROOT"]
		expect(instance.ClassName).to.equal("Folder")
		expect(instance.Name).to.equal("Parent")

		expect(#instance:GetChildren()).to.equal(0)
		expect(instanceMap:size()).to.equal(1)
	end)

	it("should fail gracefully when setting erroneous properties", function()
		local virtualInstances = {
			ROOT = {
				ClassName = "StringValue",
				Name = "Root",
				Properties = {
					Value = {
						Type = "Vector3",
						Value = { 1, 2, 3 },
					},
				},
				Children = {},
			},
		}

		local instanceMap = InstanceMap.new()
		local unappliedPatch = reify(instanceMap, virtualInstances, "ROOT")

		local instance = instanceMap.fromIds["ROOT"]
		expect(instance.ClassName).to.equal("StringValue")
		expect(instance.Name).to.equal("Root")

		assert(isEmpty(unappliedPatch.added), "expected no additions")
		expect(#unappliedPatch.updated).to.equal(1)
		assert(isEmpty(unappliedPatch.removed), "expected no removes")

		local update = unappliedPatch.updated[1]
		expect(update.id).to.equal("ROOT")
		expect(size(update.changedProperties)).to.equal(1)

		local property = update.changedProperties["Value"]
		expect(property).to.equal(virtualInstances["ROOT"].Properties.Value)
	end)

	-- This is the simplest ref case: ensure that setting a ref property that
	-- points to an instance that was previously created as part of the same
	-- reify operation works.
	it("should apply properties containing refs to ancestors", function()
		local virtualInstances = {
			ROOT = {
				ClassName = "Folder",
				Name = "Root",
				Properties = {},
				Children = { "CHILD" },
			},

			CHILD = {
				ClassName = "ObjectValue",
				Name = "Child",
				Properties = {
					Value = {
						Ref = "ROOT",
					},
				},
				Children = {},
			},
		}

		local instanceMap = InstanceMap.new()
		local unappliedPatch = reify(instanceMap, virtualInstances, "ROOT")

		assert(PatchSet.isEmpty(unappliedPatch), "expected remaining patch to be empty")

		local root = instanceMap.fromIds["ROOT"]
		local child = instanceMap.fromIds["CHILD"]
		expect(child.Value).to.equal(root)
	end)

	-- This is another simple case: apply a ref property that points to an
	-- existing instance. In this test, that instance was created before the
	-- reify operation started and is present in instanceMap.
	it("should apply properties containing refs to previously-existing instances", function()
		local virtualInstances = {
			ROOT = {
				ClassName = "ObjectValue",
				Name = "Root",
				Properties = {
					Value = {
						Ref = "EXISTING",
					},
				},
				Children = {},
			},
		}

		local instanceMap = InstanceMap.new()

		local existing = Instance.new("Folder")
		existing.Name = "Existing"
		instanceMap:insert("EXISTING", existing)

		local unappliedPatch = reify(instanceMap, virtualInstances, "ROOT")

		assert(PatchSet.isEmpty(unappliedPatch), "expected remaining patch to be empty")

		local root = instanceMap.fromIds["ROOT"]
		expect(root.Value).to.equal(existing)
	end)

	-- This is a tricky ref case: CHILD_A points to CHILD_B, but is constructed
	-- first. Deferred ref application is required to implement this case
	-- correctly.
	it("should apply properties containing refs to later siblings correctly", function()
		local virtualInstances = {
			ROOT = {
				ClassName = "Folder",
				Name = "Root",
				Properties = {},
				Children = { "CHILD_A", "CHILD_B" },
			},

			CHILD_A = {
				ClassName = "ObjectValue",
				Name = "Child A",
				Properties = {
					Value = {
						Ref = "CHILD_B",
					},
				},
				Children = {},
			},

			CHILD_B = {
				ClassName = "Folder",
				Name = "Child B",
				Properties = {},
				Children = {},
			},
		}

		local instanceMap = InstanceMap.new()
		local unappliedPatch = reify(instanceMap, virtualInstances, "ROOT")

		assert(PatchSet.isEmpty(unappliedPatch), "expected remaining patch to be empty")

		local childA = instanceMap.fromIds["CHILD_A"]
		local childB = instanceMap.fromIds["CHILD_B"]
		expect(childA.Value).to.equal(childB)
	end)

	-- This is the classic case that calls for deferred ref application. In this
	-- test, the root instance has a ref property that refers to its child. The
	-- root is definitely constructed first.
	--
	-- This is distinct from the sibling case in that the child will be
	-- constructed as part of a recursive call before the parent has totally
	-- finished. Given deferred refs, this should not fail, but it is a good
	-- case to test.
	it("should apply properties containing refs to later children correctly", function()
		local virtualInstances = {
			ROOT = {
				ClassName = "ObjectValue",
				Name = "Root",
				Properties = {
					Value = {
						Ref = "CHILD",
					},
				},
				Children = { "CHILD" },
			},

			CHILD = {
				ClassName = "Folder",
				Name = "Child",
				Properties = {},
				Children = {},
			},
		}

		local instanceMap = InstanceMap.new()
		local unappliedPatch = reify(instanceMap, virtualInstances, "ROOT")

		assert(PatchSet.isEmpty(unappliedPatch), "expected remaining patch to be empty")

		local root = instanceMap.fromIds["ROOT"]
		local child = instanceMap.fromIds["CHILD"]
		expect(root.Value).to.equal(child)
	end)

	it("should return a partial patch when applying invalid refs", function()
		local virtualInstances = {
			ROOT = {
				ClassName = "ObjectValue",
				Name = "Root",
				Properties = {
					Value = {
						Type = "Ref",
						Value = "SORRY",
					},
				},
				Children = {},
			},
		}

		local instanceMap = InstanceMap.new()
		local unappliedPatch = reify(instanceMap, virtualInstances, "ROOT")

		assert(not PatchSet.hasRemoves(unappliedPatch), "expected no removes")
		assert(not PatchSet.hasAdditions(unappliedPatch), "expected no additions")
		expect(#unappliedPatch.updated).to.equal(1)

		local update = unappliedPatch.updated[1]
		expect(update.id).to.equal("ROOT")
		expect(update.changedProperties.Value).to.equal(virtualInstances["ROOT"].Properties.Value)
	end)
end
