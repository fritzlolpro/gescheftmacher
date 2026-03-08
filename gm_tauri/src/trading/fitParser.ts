export interface FitItem {
  name: string
  qty: number
}

export interface ParsedFitGroup {
  /** null = general group (loose items pasted outside any fit) */
  fitName: string | null
  /** the ship from [ShipName, FitName], null for general group */
  shipName: string | null
  items: FitItem[]
}

export interface ParsedPaste {
  groups: ParsedFitGroup[]
}

/**
 * Parses an EVE Online fit paste (or plain item list) into groups.
 *
 * Rules:
 *  - `[ShipName, FitName]`  → new fit group; ship is included as first item
 *  - `ItemName xN`          → qty = N
 *  - `ItemName, Script`     → loaded module, take only the part before comma
 *  - Duplicate names within a group are merged (qty summed)
 *  - Items before any header go into the general (fitName=null) group
 */
export function parsePaste(text: string): ParsedPaste {
  const lines = text.split('\n')
  const groups: ParsedFitGroup[] = []
  let currentGroup: ParsedFitGroup | null = null

  function pushItem(name: string, qty: number) {
    if (!name) return
    if (!currentGroup) {
      currentGroup = { fitName: null, shipName: null, items: [] }
    }
    const existing = currentGroup.items.find(
      i => i.name.toLowerCase() === name.toLowerCase()
    )
    if (existing) {
      existing.qty += qty
    } else {
      currentGroup.items.push({ name, qty })
    }
  }

  for (const rawLine of lines) {
    const line = rawLine.trim()
    if (!line) continue

    // Fit header: [ShipName, FitName]
    const headerMatch = line.match(/^\[([^\],]+),\s*(.+)\]$/)
    if (headerMatch) {
      if (currentGroup) groups.push(currentGroup)
      const shipName = headerMatch[1].trim()
      const fitName = headerMatch[2].trim()
      currentGroup = { fitName, shipName, items: [] }
      // Ship itself is the first item
      currentGroup.items.push({ name: shipName, qty: 1 })
      continue
    }

    // xN suffix: "Item Name x100"
    const xnMatch = line.match(/^(.+?)\s+x(\d+)$/i)
    if (xnMatch) {
      const name = xnMatch[1].trim()
      const qty = parseInt(xnMatch[2], 10)
      pushItem(name, qty)
      continue
    }

    // Loaded module: "ItemName, Script" — keep only part before comma
    const commaIdx = line.indexOf(',')
    const name = commaIdx >= 0 ? line.slice(0, commaIdx).trim() : line.trim()
    pushItem(name, 1)
  }

  if (currentGroup) groups.push(currentGroup)

  return { groups }
}
