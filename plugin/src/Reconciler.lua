local Reconciler = {}

local function isInit(item, itemFileName)
	if item and item.type == "dir" then
		return
	end

	return not not itemFileName:find("^init%.")
end

local function findInit(item)
	if item.type ~= "dir" then
		return nil, nil
	end

	for childFileName, childItem in pairs(item.children) do
		if isInit(childItem, childFileName) then
			return childItem, childFileName
		end
	end

	return nil, nil
end

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
		local initItem, initFileName = findInit(item)

		if initItem then
			local rbx = Reconciler._reify(initItem, initFileName)
			rbx.Name = fileName

			return rbx
		else
			local rbx = Instance.new("Folder")
			rbx.Name = fileName

			return rbx
		end
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

function Reconciler._reify(item, fileName, parent)
	local rbx = Reconciler._reifyShallow(item, fileName)

	if item.type == "dir" then
		for childFileName, childItem in pairs(item.children) do
			if not isInit(childItem, childFileName) then
				local childRbx = Reconciler._reify(childItem, childFileName)
				childRbx.Parent = rbx
			end
		end
	end

	rbx.Parent = parent

	return rbx
end

function Reconciler.reconcile(rbx, item, fileName, parent)
	-- Item was deleted!
	if not item then
		if isInit(item, fileName) then
			if not parent then
				return
			end

			-- Un-usurp parent!
			local newParent = Instance.new("Folder")
			newParent.Name = parent.Name

			for _, child in ipairs(parent:GetChildren()) do
				child.Parent = newParent
			end

			newParent.Parent = parent.Parent
			parent:Destroy()

			return
		else
			if rbx then
				rbx:Destroy()
			end

			return
		end
	end

	if item.type == "dir" then
		-- Folder was created!
		if not rbx then
			return Reconciler._reify(item, fileName, parent)
		end

		local initItem, initFileName = findInit(item)

		if initItem then
			local _, initClassName = itemToName(initItem, initFileName)

			if rbx.ClassName == initClassName then
				setValues(rbx, initItem, initFileName)
			else
				rbx:Destroy()
				return Reconciler._reify(item, fileName, parent)
			end
		else
			if rbx.ClassName ~= "Folder" then
				rbx:Destroy()
				return Reconciler._reify(item, fileName, parent)
			end
		end

		local visitedChildren = {}

		for childFileName, childItem in pairs(item.children) do
			if not isInit(childItem, childFileName) then
				local childName = itemToName(childItem, childFileName)

				visitedChildren[childName] = true

				Reconciler.reconcile(rbx:FindFirstChild(childName), childItem, childFileName, rbx)
			end
		end

		for _, childRbx in ipairs(rbx:GetChildren()) do
			-- Child was deleted!
			if not visitedChildren[childRbx.Name] then
				Reconciler.reconcile(childRbx, nil, nil)
			end
		end

		return rbx
	elseif item.type == "file" then
		if isInit(item, fileName) then
			-- Usurp our container!
			local _, className = itemToName(item, fileName)

			if parent.ClassName == className then
				rbx = parent
			else
				rbx = Reconciler._reify(item, fileName, parent.Parent)
				rbx.Name = parent.Name

				for _, child in ipairs(parent:GetChildren()) do
					child.Parent = rbx
				end

				parent:Destroy()
			end

			setValues(rbx, item, fileName)

			return rbx
		else
			if not rbx then
				return Reconciler._reify(item, fileName, parent)
			end

			local _, className = itemToName(item, fileName)

			if rbx.ClassName ~= className then
				rbx:Destroy()
				return Reconciler._reify(item, fileName, parent)
			end

			setValues(rbx, item, fileName)

			return rbx
		end
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
	Reconciler.reconcile(rbx, item, fileName, location)
end

return Reconciler
