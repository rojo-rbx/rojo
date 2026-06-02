return function()
	local orderSwaps = require(script.Parent.orderSwaps)

	it("orders same-named siblings by their original GetChildren order", function()
		local parent = Instance.new("Model")
		local a1 = Instance.new("Part")
		a1.Name = "a"
		a1.Parent = parent
		local a2 = Instance.new("Part")
		a2.Name = "a"
		a2.Parent = parent
		local a3 = Instance.new("Part")
		a3.Name = "a"
		a3.Parent = parent

		-- Input deliberately out of sibling order.
		-- orderSwaps must restore the GetChildren() order.
		local ordered = orderSwaps({
			{ id = "3", oldInstance = a3 },
			{ id = "1", oldInstance = a1 },
			{ id = "2", oldInstance = a2 },
		})

		expect(ordered[1].oldInstance).to.equal(a1)
		expect(ordered[2].oldInstance).to.equal(a2)
		expect(ordered[3].oldInstance).to.equal(a3)
	end)

	it("orders ancestors before descendants", function()
		local root = Instance.new("Model")
		local child = Instance.new("Folder")
		child.Parent = root
		local grandchild = Instance.new("Part")
		grandchild.Parent = child

		local ordered = orderSwaps({
			{ id = "grandchild", oldInstance = grandchild },
			{ id = "child", oldInstance = child },
			{ id = "root", oldInstance = root },
		})

		expect(ordered[1].oldInstance).to.equal(root)
		expect(ordered[2].oldInstance).to.equal(child)
		expect(ordered[3].oldInstance).to.equal(grandchild)
	end)

	it("returns a single swap unchanged", function()
		local part = Instance.new("Part")

		local ordered = orderSwaps({
			{ id = "1", oldInstance = part },
		})

		expect(#ordered).to.equal(1)
		expect(ordered[1].oldInstance).to.equal(part)
	end)
end
