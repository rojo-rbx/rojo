return function()
	local CollectionService = game:GetService("CollectionService")
	local Inspect = require(script.Parent.Inspect)
	local InstanceReferences = require(script.Parent.Parent.InstanceReferences)

	local root
	local function request(depth, maxChildren, maxInstances)
		return {
			kind = "inspect",
			target = { kind = "path", segments = { "Workspace", root.Name } },
			depth = depth,
			maxChildren = maxChildren or 100,
			maxInstances = maxInstances or 2_000,
			includeProperties = true,
			includeAttributes = true,
			includeTags = true,
		}
	end

	describe("Inspect handler", function()
		beforeEach(function()
			root = Instance.new("Folder")
			root.Name = "PrismInspectSpec"
			root.Parent = workspace
		end)

		afterEach(function()
			root:Destroy()
		end)

		it("resolves roots, children, and quoted diagnostics", function()
			local child = Instance.new("Part")
			child.Name = "Name.With.Dots"
			child.Parent = root
			local result = Inspect.run(request(1), { references = InstanceReferences.new("session") })
			expect(result.root.children[1].path:find('"Name.With.Dots"', 1, true)).to.be.ok()
			expect(result.root.children[1].className).to.equal("Part")
		end)

		it("honors depth, child, and total limits", function()
			for index = 1, 3 do
				local child = Instance.new("Folder")
				child.Name = tostring(index)
				child.Parent = root
			end
			local depthZero = Inspect.run(request(0), { references = InstanceReferences.new("session") })
			expect(#depthZero.root.children).to.equal(0)
			local childLimited = Inspect.run(request(1, 1), { references = InstanceReferences.new("session") })
			expect(#childLimited.root.children).to.equal(1)
			expect(childLimited.truncated).to.equal(true)
			local totalLimited = Inspect.run(request(1, 100, 1), { references = InstanceReferences.new("session") })
			expect(totalLimited.truncationReason).to.equal("maxInstances")
		end)

		it("sorts attributes and tags and uses the property allowlist", function()
			root:SetAttribute("Zed", 1)
			root:SetAttribute("Alpha", true)
			CollectionService:AddTag(root, "Zulu")
			CollectionService:AddTag(root, "Alpha")
			local result = Inspect.run(request(0), { references = InstanceReferences.new("session") })
			expect(result.root.attributes.Alpha.kind).to.equal("boolean")
			expect(result.root.tags[1]).to.equal("Alpha")
			expect(result.root.properties.Archivable.kind).to.equal("boolean")
			expect(result.root.properties.Source).never.to.be.ok()
		end)

		it("reports missing and destroyed targets", function()
			local missing = request(0)
			missing.target.segments[2] = "MissingPrismInspectTarget"
			expect(select(1, Inspect.run(missing, { references = InstanceReferences.new("session") }))).never.to.be.ok()
			root:Destroy()
			expect(select(1, Inspect.run(request(0), { references = InstanceReferences.new("session") }))).never.to.be.ok()
		end)
	end)
end
