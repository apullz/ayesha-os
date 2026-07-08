export type Rank = "clade" | "class" | "family" | "genus" | "species";

export interface PlantNode {
  name: string; // e.g. "Plantae", "Bryophytes", "Ericaceae", "Calluna", "vulgaris"
  rank: Rank;
  commonName?: string;
  gaelicName?: string;
  geologicalEra?: string; // e.g., "Devonian, ~410 Mya"
  evolutionaryMilestone?: string; // e.g., "Evolution of vascular tissues"
  description: string;
  habitat?: string;
  lore?: string;
  status?: string; // e.g., "Common", "Endemic relic", "Vulnerable"
  asciiArt?: string;
  children?: Record<string, PlantNode>; // sub-directories
}
