--[[
	Defines the process of "hydration" -- matching up a virtual DOM with
	concrete instances and assigning them IDs.
]]

local invariant = require(script.Parent.Parent.invariant)

local function hydrate(instanceMap, virtualInstances, rootId, rootInstance)
	local virtualInstance = virtualInstances[rootId]

	if virtualInstance == nil then
		invariant("Cannot hydrate an instance not present in virtualInstances\nID: {}", rootId)
	end

	instanceMap:insert(rootId, rootInstance)

	local existingChildren = rootInstance:GetChildren()

	-- For each existing child, we'll track whether it's been paired with an
	-- instance that the Rojo server knows about.
	local isExistingChildVisited = {}
	for i = 1, #existingChildren do
		isExistingChildVisited[i] = false
	end

	for _, childId in ipairs(virtualInstance.Children) do
		local virtualChild = virtualInstances[childId]

		for childIndex, childInstance in ipairs(existingChildren) do
			if not isExistingChildVisited[childIndex] then
				-- We guard accessing Name and ClassName in order to avoid
				-- tripping over children of DataModel that Rojo won't have
				-- permissions to access at all.
				local accessSuccess, name, className = pcall(function()
					return childInstance.Name, childInstance.ClassName
				end)

				-- This rule is very conservative and could be loosened in the
				-- future, or more heuristics could be introduced.
				if accessSuccess and name == virtualChild.Name and className == virtualChild.ClassName then
					isExistingChildVisited[childIndex] = true
					hydrate(instanceMap, virtualInstances, childId, childInstance)
					break
				end
			end
		end
	end
end

return hydrate
