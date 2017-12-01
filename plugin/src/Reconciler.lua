local Reconciler = {}

local function itemToName(item, fileName)
	if item and item.type == "dir" then
		return fileName, "Folder"
	elseif item and item.type == "file" or not item then
		if fileName:find("%.server%.lua$") then
			return fileName:match("^(.-)%.server%.lua$"), "Script"
		elseif fileName:find("%.client%.lua$") then
			return fileName:match("^(.-)%.client%.lua$"), "LocalScript"
		elseif fileName:find("%.lua") then
			return fileName:match("^(.-)%.lua$"), "ModuleScript"
		else
			return fileName, "StringValue"
		end
	else
		error("unknown item type " .. tostring(item.type))
	end
end

local function setValues(rbx, item, fileName)
	local _, className = itemToName(item, fileName)

	if className:find("Script") then
		rbx.Source = item.contents
	else
		rbx.Value = item.contents
	end
end

function Reconciler._reifyShallow(item, fileName)
	if item.type == "dir" then
		-- TODO: handle init
		local rbx = Instance.new("Folder")
		rbx.Name = fileName

		return rbx
	elseif item.type == "file" then
		local objectName, className = itemToName(item, fileName)

		local rbx = Instance.new(className)
		rbx.Name = objectName

		setValues(rbx, item, fileName)

		return rbx
	else
		error("unknown item type " .. tostring(item.type))
	end
end

function Reconciler._reify(item, fileName)
	local rbx = Reconciler._reifyShallow(item, fileName)

	if item.type == "dir" then
		for childName, child in pairs(item.children) do
			local childRbx = Reconciler._reify(child, childName)
			childRbx.Parent = rbx

			-- TODO: handle init
		end
	end

	return rbx
end

function Reconciler.reconcile(rbx, item, fileName)
	-- Item was deleted!
	if not item then
		if rbx then
			rbx:Destroy()
		end

		return
	end

	-- Item was created!
	if not rbx then
		return Reconciler._reify(item, fileName)
	end

	if item.type == "dir" then
		-- TODO: handle init

		if rbx.ClassName ~= "Folder" then
			return Reconciler._reify(item, fileName)
		end

		local visitedChildren = {}

		for childFileName, childItem in pairs(item.children) do
			local childName = itemToName(childItem, childFileName)

			visitedChildren[childName] = true

			Reconciler.reconcile(rbx:FindFirstChild(childName), childItem, childFileName)
		end

		for _, childRbx in ipairs(rbx:GetChildren()) do
			-- Child was deleted!
			if not visitedChildren[childRbx.Name] then
				Reconciler.reconcile(childRbx, nil, nil)
			end
		end

		return rbx
	elseif item.type == "file" then
		local _, className = itemToName(item, fileName)

		if rbx.ClassName ~= className then
			return Reconciler._reify(item, fileName)
		end

		setValues(rbx, item, fileName)

		return rbx
	else
		error("unknown item type " .. tostring(item.type))
	end
end

function Reconciler.reconcileRoute(route, item)
	local location = game

	for i = 1, #route - 1 do
		local piece = route[i]
		local newLocation = location:FindFirstChild(piece)

		if not newLocation then
			newLocation = Instance.new("Folder")
			newLocation.Name = piece
			newLocation.Parent = location
		end

		location = newLocation
	end

	local fileName = route[#route]

	local name = itemToName(item, fileName)
	local rbx = location:FindFirstChild(name)
	local newRbx = Reconciler.reconcile(rbx, item, fileName)

	if newRbx ~= rbx then
		if rbx then
			rbx:Destroy()
		end

		if newRbx then
			newRbx.Parent = location
		end
	end
end

return Reconciler
