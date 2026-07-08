import { PlantNode } from "../types";

export const floraData: PlantNode = {
  name: "Plantae",
  rank: "clade",
  commonName: "Plants",
  geologicalEra: "Precambrian to Phanerozoic, ~1.2 Bya",
  evolutionaryMilestone: "Eukaryotic photosynthesis via primary endosymbiosis (plastids)",
  description: "The primary kingdom of photosynthetic multicellular eukaryotes, spanning from ancient unicellular green algae to massive conifers and complex flowering plants.",
  children: {
    bryophytes: {
      name: "Bryophytes",
      rank: "clade",
      commonName: "Non-vascular Plants",
      geologicalEra: "Ordovician, ~470 Mya",
      evolutionaryMilestone: "First land plants; transition from aquatic to terrestrial life, evolution of a protective waxy cuticle and sporopollenin-coated spores.",
      description: "An informal group consisting of three divisions of non-vascular land plants: liverworts, hornworts, and mosses. Lacking specialized vascular tissues (xylem and phloem), they absorb water and nutrients directly through their leaves and rely on water for swimming sperm.",
      children: {
        mosses: {
          name: "Bryophyta",
          rank: "class",
          commonName: "Mosses",
          geologicalEra: "Ordovician, ~450 Mya",
          evolutionaryMilestone: "Development of simple multicellular rhizoids and leaf-like structures (microphylls) to survive dry spells.",
          description: "Small, non-vascular flowerless plants that typically form dense green clumps or mats, often in damp or shady locations. Extremely prevalent and ecologically vital in Scotland's peat bogs, mountain heaths, and humid temperate rainforests.",
          children: {
            sphagnaceae: {
              name: "Sphagnaceae",
              rank: "family",
              commonName: "Peat Moss Family",
              geologicalEra: "Carboniferous, ~350 Mya",
              evolutionaryMilestone: "Evolution of specialized large hyaline cells with water-holding pores (holding up to 20x their dry weight).",
              description: "A highly specialized family of mosses that dominate boggy landscapes. They play a monumental role in global carbon sequestration by forming layers of peat in cool, wet climates like northern and western Scotland.",
              children: {
                sphagnum: {
                  name: "Sphagnum",
                  rank: "genus",
                  commonName: "Peat Moss",
                  description: "The primary builder of Scotland's bogs. By releasing hydrogen ions, they actively acidify their environment, creating conditions that prevent decay and lead to massive peat accumulation over millennia.",
                  children: {
                    capillifolium: {
                      name: "Sphagnum capillifolium",
                      rank: "species",
                      commonName: "Red Bog Moss / Acute-leaved Peat Moss",
                      gaelicName: "Còinnich-mhònadh (Red Moss)",
                      geologicalEra: "Neogene, ~15 Mya",
                      evolutionaryMilestone: "Red anthocyanin pigment adaptation providing ultraviolet protection and thermal regulation in open peatlands.",
                      description: "A beautiful, dome-forming moss that ranges from pale green to vibrant crimson. It is a critical component of healthy, active blanket bogs across the Scottish Highlands.",
                      habitat: "Acidic peat bogs, wet blanket mires, heathlands, and high altitude hummocks.",
                      lore: "Sphagnum is extremely acidic and holds natural antiseptic qualities. Historically in the Highlands, it was packed into open wounds during clan battles and later harvested en masse during World War I to serve as surgical dressings.",
                      status: "Common; key builder of Scottish carbon sinks.",
                      asciiArt: `
     .:::.   .:::.
    :::::::.:::::::
    :::::::::::::::
     ':::::::::::'
       ':::::::'
         ':::'
          |||
          |||
          |||
         //\\\\\\
        //  \\\\\\
                      `
                    }
                  }
                }
              }
            },
            polytrichaceae: {
              name: "Polytrichaceae",
              rank: "family",
              commonName: "Haircap Moss Family",
              geologicalEra: "Permian, ~280 Mya",
              evolutionaryMilestone: "Development of primitive water-conducting cells (hydroids) and photosynthetic lamellae on leaves.",
              description: "A primitive family of mosses with highly structured, rigid leaves and complex spore capsules covered by a hairy cap.",
              children: {
                polytrichum: {
                  name: "Polytrichum",
                  rank: "genus",
                  commonName: "Haircap Moss / Pigeon Wheat",
                  description: "A genus of large, rigid mosses resembling tiny conifer seedlings, with a highly developed water transport system compared to other mosses.",
                  children: {
                    commune: {
                      name: "Polytrichum commune",
                      rank: "species",
                      commonName: "Common Haircap Moss / Great Goldilocks",
                      gaelicName: "Còinnich-rìoghail (Royal Moss)",
                      geologicalEra: "Neogene, ~10 Mya",
                      evolutionaryMilestone: "Exceptionally tall growth forms (up to 30cm) supported by primitive vascular-like structural columns.",
                      description: "One of the tallest mosses in the world, forming dark blue-green carpets in Scottish forests and damp moorlands.",
                      habitat: "Damp woodlands, acidic wetlands, wet heaths, and forest margins.",
                      lore: "Highlanders historically used this tough, flexible moss to weave exceptionally durable ropes, door mats, and even soft, springy mattresses, which were prized for their resistance to dampness and rot.",
                      status: "Common and widespread in wet soils.",
                      asciiArt: `
          \\\\ | //
         ---\\|//---
         ---//|\\\\---
          // || \\\\
          \\\\ || //
           |||||
           |||||
           |||||
           |||||
           |||||
                      `
                    }
                  }
                }
              }
            }
          }
        },
        liverworts: {
          name: "Marchantiophyta",
          rank: "class",
          commonName: "Liverworts",
          geologicalEra: "Ordovician, ~470 Mya",
          evolutionaryMilestone: "Extremely primitive plant form, lacking stomata completely (utilizing simple open air pores instead).",
          description: "An ancient lineage of simple non-vascular plants, characterized by either a flat, liver-shaped body (thallose) or a leafy green appearance. Scotland's wet West Coast (the Celtic Rainforest) is a global hotspot for liverwort diversity.",
          children: {
            herbertaceae: {
              name: "Herbertaceae",
              rank: "family",
              commonName: "Prongwort Family",
              geologicalEra: "Triassic, ~220 Mya",
              evolutionaryMilestone: "Deeply bifurcated (two-pronged) leaves adapted to catch atmospheric moisture from coastal sea mists.",
              description: "A family of leafy liverworts with distinctive split leaves, restricted to highly oceanic, humid climates.",
              children: {
                herbertus: {
                  name: "Herbertus",
                  rank: "genus",
                  commonName: "Prongwort",
                  description: "A leafy liverwort genus that forms reddish-brown cushions on wet mountain rocks in hyper-humid zones.",
                  children: {
                    borealis: {
                      name: "Herbertus borealis",
                      rank: "species",
                      commonName: "Northern Prongwort / Beinn Eighe Prongwort",
                      gaelicName: "Snàthad-fhraoich mhòr (Great Heather Needle)",
                      geologicalEra: "Pleistocene Relic, ~2 Mya",
                      evolutionaryMilestone: "Hyper-adaptation to the cool, oceanic 'temperate rainforest' microclimates of Northwest Scotland.",
                      description: "An incredibly rare leafy liverwort with warm orange-brown cushions. It is a prized biogeographical treasure of Scotland.",
                      habitat: "Damp, mossy dwarf-shrub heath on steep north-facing mountain slopes.",
                      lore: "This species is of global botanical significance. In Europe, it is found almost exclusively in Scotland, with its main stronghold on the slopes of the Beinn Eighe National Nature Reserve in Torridon.",
                      status: "Extremely Rare; protected Scottish endemic relic.",
                      asciiArt: `
         \\  /  \\  /
          \\/    \\/
          (      )
           \\    /
            \\  /
             ||
             ||
            /  \\
           /    \\
                      `
                    }
                  }
                }
              }
            }
          }
        }
      }
    },
    pteridophytes: {
      name: "Pteridophytes",
      rank: "clade",
      commonName: "Spore-bearing Vascular Plants",
      geologicalEra: "Silurian to Devonian, ~420 Mya",
      evolutionaryMilestone: "First development of true vascular tissues (xylem and phloem) for long-distance transport, enabling tall upright growth.",
      description: "Vascular plants that reproduce and disperse via spores rather than seeds. They dominated Earth during the Carboniferous, forming the massive forests that later became our coal deposits. In modern Scotland, they are represented by ferns and horsetails.",
      children: {
        horsetails: {
          name: "Equisetopsida",
          rank: "class",
          commonName: "Horsetails",
          geologicalEra: "Devonian, ~380 Mya",
          evolutionaryMilestone: "Evolution of hollow, jointed stems reinforced with abrasive biogenic silica for structural rigidity.",
          description: "An ancient class of plants with segmented, ribbed stems and whorled leaves, once growing as towering trees (Calamites) during the coal-forming eras.",
          children: {
            equisetaceae: {
              name: "Equisetaceae",
              rank: "family",
              commonName: "Horsetail Family",
              geologicalEra: "Carboniferous, ~300 Mya",
              evolutionaryMilestone: "Survival of a single genus (Equisetum) from a massive, ancient group of tree-sized organisms.",
              description: "The sole surviving family of the class Equisetopsida, featuring highly silica-rich and structurally segmented hollow reeds.",
              children: {
                equisetum: {
                  name: "Equisetum",
                  rank: "genus",
                  commonName: "Horsetail",
                  description: "Primitive vascular plants with segmented stems and reproductive spores carried in terminal cone-like structures (strobili).",
                  children: {
                    sylvaticum: {
                      name: "Equisetum sylvaticum",
                      rank: "species",
                      commonName: "Wood Horsetail",
                      gaelicName: "Earball-eich (Horse's Tail)",
                      geologicalEra: "Paleogene, ~40 Mya",
                      evolutionaryMilestone: "Highly branched, compound whorls that maximize light absorption in dense, damp woodland canopies.",
                      description: "The most delicate and beautiful of the native horsetails, resembling a miniature green pine tree or a lacy fan.",
                      habitat: "Damp woodlands, forest streams, marshy meadows, and Highland river banks.",
                      lore: "Because of its exceptionally high silica content, wood horsetail was historically used in Scotland as 'shave-grass' for scrubbing brass, polishing pewter, and finishing wood instruments.",
                      status: "Common in damp, shaded Highland valleys.",
                      asciiArt: `
            /|\\
           /|||\\
          //|||\\\\
         ///|||\\\\\\
           |||
          //|||\\\\
         ///|||\\\\\\
        ////|||\\\\\\\\
           |||
           |||
                      `
                    }
                  }
                }
              }
            }
          }
        },
        ferns: {
          name: "Polypodiopsida",
          rank: "class",
          commonName: "Ferns",
          geologicalEra: "Devonian, ~360 Mya",
          evolutionaryMilestone: "Development of large, complex leaves with branching veins (megaphylls/fronds) and subterranean roots.",
          description: "A diverse group of vascular spore-bearing plants. Their leaves emerge as rolled-up coils known as crosiers or 'fiddleheads', unfolding in spectacular geometrical symmetry.",
          children: {
            dennstaedtiaceae: {
              name: "Dennstaedtiaceae",
              rank: "family",
              commonName: "Bracken Family",
              geologicalEra: "Cretaceous, ~100 Mya",
              evolutionaryMilestone: "Evolution of deep, fire-resistant underground creeping rootstocks (rhizomes) allowing swift colonization.",
              description: "A cosmopolitan family of large ferns, heavily dominant in woodlands and open hillsides.",
              children: {
                pteridium: {
                  name: "Pteridium",
                  rank: "genus",
                  commonName: "Bracken",
                  description: "Aggressive, tall, coarse ferns with tri-pinnate leaves that form extensive clonal colonies on Scottish hillsides.",
                  children: {
                    aquilinum: {
                      name: "Pteridium aquilinum",
                      rank: "species",
                      commonName: "Bracken / Bracken Fern",
                      gaelicName: "Raineach-mhòr (Great Fern)",
                      geologicalEra: "Neogene, ~15 Mya",
                      evolutionaryMilestone: "Production of chemical feeding deterrents (cyanogenic glycosides) making it toxic to herbivores.",
                      description: "A massive, hardy fern that dominates wide swathes of the Scottish Highland hillsides. While native, its aggressive colonization can crowd out delicate native flora.",
                      habitat: "Open hillsides, moorland edges, open birch woodlands, and sandy soils.",
                      lore: "In medieval Scotland, bracken was highly valued. It was harvested for animal bedding, thatch for blackhouses, and burned to ash to create potash, a key ingredient for Scotland's early soap and glass industries.",
                      status: "Abundant; highly aggressive colonist.",
                      asciiArt: `
            \\ /
          *--o--*
         /  / \\  \\
        *--o---o--*
       /  /     \\  \\
      *--o-------o--*
        /         \\
       /           \\
      /             \\
                      `
                    }
                  }
                }
              }
            }
          }
        }
      }
    },
    gymnosperms: {
      name: "Gymnosperms",
      rank: "clade",
      commonName: "Conifers & Allies / Naked Seeds",
      geologicalEra: "Carboniferous to Devonian, ~370 Mya",
      evolutionaryMilestone: "Revolutionary evolution of the seed, freeing plant reproduction from the requirement of standing water.",
      description: "Seed-producing vascular plants whose seeds are 'naked'—not enclosed within a protective fruit or ovary, but typically borne on the scales of cones.",
      children: {
        pinaceae: {
          name: "Pinaceae",
          rank: "family",
          commonName: "Pine Family",
          geologicalEra: "Triassic, ~220 Mya",
          evolutionaryMilestone: "Development of specialized resin ducts to seal wounds and defend against boring insects and fungal rot.",
          description: "An incredibly successful family of evergreen conifers with needle-like leaves and woody cones, adapted to withstand extreme freezing and nutrient-poor soils.",
          children: {
            pinus: {
              name: "Pinus",
              rank: "genus",
              commonName: "Pine",
              description: "A genus of resinous, cone-bearing trees with needles arranged in bundles (fascicles) of two, three, or five.",
              children: {
                sylvestris: {
                  name: "Pinus sylvestris",
                  rank: "species",
                  commonName: "Scots Pine",
                  gaelicName: "Giuthas (Pine)",
                  geologicalEra: "Miocene, ~20 Mya",
                  evolutionaryMilestone: "Extreme freeze-tolerance via cellular dehydration and custom anti-freeze sugars, perfectly suited for the boreal ice margins.",
                  description: "The national tree of Scotland and the magnificent keystone species of the ancient, wild Caledonian Forest. It features beautiful orange-red bark on its upper trunk.",
                  habitat: "Acidic, sandy, and nutrient-poor soils, dry heathlands, and rugged Highland glen slopes.",
                  lore: "In Highland history, the Scots Pine was invaluable. Known as 'the resinous fire of the north', its rich heartwood split into 'candle-fir' strips to illuminate dark winter evenings. Strips were also used for clan beacons, ship masts, and as grave markers of high-ranking chiefs.",
                  status: "Locally Endangered Caledonian Forest icon; protected.",
                  asciiArt: `
          _\\\\//_
         _\\\\||//_
        _\\\\\\||///_
       __\\\\\\||///__
         |||||||
         |||||||
         |||||||
         |||||||
        /|||||||\\
                      `
                    }
                  }
                }
              }
            },
            cupressaceae: {
              name: "Cupressaceae",
              rank: "family",
              commonName: "Cypress Family",
              geologicalEra: "Triassic, ~210 Mya",
              evolutionaryMilestone: "Scale-like overlapping leaves and cones with fused scales, maximizing drought and wind resistance.",
              description: "A cosmopolitan conifer family with fibrous bark, scale-like or needle-shaped leaves, and woody or berry-like cones.",
              children: {
                juniperus: {
                  name: "Juniperus",
                  rank: "genus",
                  commonName: "Juniper",
                  description: "A genus of highly aromatic evergreen shrubs and trees whose female cones have fleshy, berry-like scales used as a spice.",
                  children: {
                    communis: {
                      name: "Juniperus communis",
                      rank: "species",
                      commonName: "Common Juniper",
                      gaelicName: "Aiteann (The Sharp Bush)",
                      geologicalEra: "Miocene, ~10 Mya",
                      evolutionaryMilestone: "Fleshy, berry-like female cone evolution designed for dispersal by birds in open montane climates.",
                      description: "A low-growing, prickly conifer with silver-striped needles and deep purple-blue berries that take two years to fully ripen.",
                      habitat: "Highland pinewoods, rocky mountain slopes, and moorlands.",
                      lore: "Juniper wood burns with a sweet, aromatic, and nearly invisible smoke. Highlanders burned it during 'saining' rituals to purify their homes and cattle. It was also favored by illicit whiskey distillers because the lack of smoke helped keep their hidden stills secret from the tax collectors. Today, it is harvested to flavor artisanal Scottish craft gins.",
                      status: "Vulnerable in Scotland due to disease and overgrazing.",
                      asciiArt: `
         *  /\\  *
          */  \\*
         **|##|**
         **|##|**
         **|##|**
           /||\\
          / || \\
                      `
                    }
                  }
                }
              }
            },
            taxaceae: {
              name: "Taxaceae",
              rank: "family",
              commonName: "Yew Family",
              geologicalEra: "Jurassic, ~180 Mya",
              evolutionaryMilestone: "Adaptation of poisonous alkaloids (taxanes) throughout the tissues, creating near-absolute defense against herbivores.",
              description: "A small family of evergreen conifers with flat, dark green needles and seeds enclosed in a bright red, cup-like fleshy sheath called an aril.",
              children: {
                taxus: {
                  name: "Taxus",
                  rank: "genus",
                  commonName: "Yew",
                  description: "Incredibly long-lived, slow-growing conifers, famous for their rot-resistant wood and association with sacred ancient sites.",
                  children: {
                    baccata: {
                      name: "Taxus baccata",
                      rank: "species",
                      commonName: "European Yew / English Yew",
                      gaelicName: "Iubhar (The Living Bow)",
                      geologicalEra: "Miocene, ~12 Mya",
                      evolutionaryMilestone: "Fleshy red aril seed coating to attract birds for seed dispersal, bypassing the poison barrier.",
                      description: "An legendary evergreen tree that can live for thousands of years, regenerating from within its own hollowed-out trunks.",
                      habitat: "Dry limestone cliffs, shaded oakwoods, and ancient churchyards.",
                      lore: "The Fortingall Yew in Perthshire, Scotland, is estimated to be between 2,000 and 5,000 years old, making it one of the oldest living organisms in Europe. Yew was the ultimate wood for crafting deadly medieval longbows. To the Gaels, the Yew was the tree of eternity, representing the endless cycle of life, death, and rebirth.",
                      status: "Common but individual ancient specimens are highly protected.",
                      asciiArt: `
        () .--. ()
          _)(__(_
         (o)(o)(o)
          \\||||/
           ||||
           ||||
           ||||
          /||||\\
                      `
                    }
                  }
                }
              }
            }
          }
        },
    angiosperms: {
      name: "Angiosperms",
      rank: "clade",
      commonName: "Flowering Plants",
      geologicalEra: "Jurassic to Cretaceous, ~140 Mya",
      evolutionaryMilestone: "The massive evolutionary innovation of the flower, co-opting animal pollinators for highly efficient targeted breeding.",
      description: "The most diverse group of land plants on Earth, bearing seeds enclosed within a protective ovary or fruit. Their rapid expansion radically altered terrestrial ecosystems worldwide.",
      children: {
        monocots: {
          name: "Monocotyledons",
          rank: "clade",
          commonName: "Monocots",
          geologicalEra: "Cretaceous, ~120 Mya",
          evolutionaryMilestone: "Evolution of a single embryonic seed leaf (cotyledon), parallel leaf venation, and scattered vascular bundles.",
          description: "A major clade of flowering plants that includes grasses, lilies, orchids, and palms. They generally lack secondary woody growth and have fibrous root systems.",
          children: {
            orchidaceae: {
              name: "Orchidaceae",
              rank: "family",
              commonName: "Orchid Family",
              geologicalEra: "Cretaceous, ~80 Mya",
              evolutionaryMilestone: "Exquisite bilateral symmetry of flowers, fused reproductive structures (column), and dust-like seeds requiring symbiotic fungi.",
              description: "One of the two largest families of flowering plants, world-famous for their complex, highly specialized floral structures and intimate relationships with pollinators.",
              children: {
                dactylorhiza: {
                  name: "Dactylorhiza",
                  rank: "genus",
                  commonName: "Marsh Orchid",
                  description: "Terrestrial orchids with hand-like tubers, deeply lobed or spotted leaves, and dense spires of purple-pink flowers.",
                  children: {
                    purpurella: {
                      name: "Dactylorhiza purpurella",
                      rank: "species",
                      commonName: "Northern Marsh Orchid",
                      gaelicName: "Urach-bhallach (Spotted Lily / Love Tuber)",
                      geologicalEra: "Pleistocene, ~1.5 Mya",
                      evolutionaryMilestone: "Fungal symbiosis (Mycorrhiza) allowing micro-seeds to grow without any food reserves.",
                      description: "A stunning, stout orchid with unspotted dark green leaves and dense spikes of intense purple-magenta flowers.",
                      habitat: "Damp marshes, marshy dunes, wet meadows, and coastal machair sand dunes of Scotland's outer islands.",
                      lore: "Because of their finger-lobed tubers, which resemble tiny hands, they were historically known in Scotland as 'The Devil's Hand' or 'The Hand of Mary'. They were believed to hold magical powers over fertility, love, and fate.",
                      status: "Locally abundant, but strictly protected under conservation laws.",
                      asciiArt: `
           (V)
          ((V))
         (((V)))
        ((((V))))
          |||||
         /|||||\\
        /  |||  \\
       /   |||   \\
                      `
                    }
                  }
                }
              }
            },
            poaceae: {
              name: "Poaceae",
              rank: "family",
              commonName: "Grass Family",
              geologicalEra: "Cretaceous, ~70 Mya",
              evolutionaryMilestone: "Basal meristems (growing from the ground up rather than tips) allowing rapid recovery from grazing and fire.",
              description: "The most economically and ecologically important family of plants on Earth, dominant in meadows, prairies, and dunes.",
              children: {
                ammophila: {
                  name: "Ammophila",
                  rank: "genus",
                  commonName: "Marram Grass / Bent Grass",
                  description: "Coarse, wind-tolerant grasses with deep, extensive creeping root systems that bind sand dunes together.",
                  children: {
                    arenaria: {
                      name: "Ammophila arenaria",
                      rank: "species",
                      commonName: "Marram Grass / European Beachgrass",
                      gaelicName: "Muran (The Binder)",
                      geologicalEra: "Pleistocene, ~2 Mya",
                      evolutionaryMilestone: "Extreme tolerance to salt spray and shifting sand, actively multiplying when buried under fresh sand.",
                      description: "A tall, tough grass with needle-sharp, rolled grey-green leaves that form deep, stabilizing root networks in sand.",
                      habitat: "Coastal sand dunes, machairs, and sandy maritime coastlines of Scotland.",
                      lore: "Marram grass is a legendary coastal defender. Its roots bind loose sands, creating the protective dune systems that shield Scottish coastal communities and 'machair' pastures from the ferocious Atlantic storms. Historically, harvesting marram was strictly controlled by Scottish law to prevent erosion; the leaves were harvested to weave horse collars, baskets, and thatch.",
                      status: "Abundant and critical for coastal defense.",
                      asciiArt: `
         \\  |  /
          \\ | /
         __\\|/__
           |#|
           |#|
           |#|
          /|#|\\
         / |#| \\
                      `
                    }
                  }
                }
              }
            }
          }
        },
        eudicots: {
          name: "Eudicotyledons",
          rank: "clade",
          commonName: "True Dicots",
          geologicalEra: "Cretaceous, ~125 Mya",
          evolutionaryMilestone: "Pollen grains with three distinct apertures (tricolpate pollen), paired embryonic leaves, and ringed vascular structures.",
          description: "The largest group of flowering plants, featuring diverse structures, broad leaves with net-like veins, and complex floral organs.",
          children: {
            ericaceae: {
              name: "Ericaceae",
              rank: "family",
              commonName: "Heather / Heath Family",
              geologicalEra: "Cretaceous, ~90 Mya",
              evolutionaryMilestone: "Symbiotic ericoid mycorrhizal fungi allowing survival in severely acidic, nutrient-deficient peat soils.",
              description: "A family of woody, acid-loving shrubs that dominate low-nutrient environments such as bogs, pine forests, and rocky mountain slopes.",
              children: {
                calluna: {
                  name: "Calluna",
                  rank: "genus",
                  commonName: "Heather / Ling",
                  description: "A monospecific genus represented solely by Calluna vulgaris, distinguished from other heathers by its deeply divided calyx.",
                  children: {
                    vulgaris: {
                      name: "Calluna vulgaris",
                      rank: "species",
                      commonName: "Heather / Ling Heather",
                      gaelicName: "Fraoch (The Heather)",
                      geologicalEra: "Miocene, ~8 Mya",
                      evolutionaryMilestone: "Minute, overlapping scale-like leaves and highly tough woody tissue to withstand gale-force winds.",
                      description: "The iconic, rugged, purple-flowered evergreen shrub that blankets millions of acres of the Scottish Highlands, coloring the mountains in late summer.",
                      habitat: "Acidic moorlands, dry heathlands, peat bogs, and pinewood floor clearings.",
                      lore: "Heather is deeply woven into Scottish identity. It was historically used to brew heather ale, build brooms, weave ropes, thatch roofs, and stuff extremely comfortable, aromatic mattresses. White heather is highly prized as a legendary symbol of luck, allegedly first popularized when Malvina, daughter of the ancient Gaelic bard Ossian, wept over her fallen lover, turning the purple heather beneath her tears to pure white.",
                      status: "Abundant; the visual and ecological definition of the Highlands.",
                      asciiArt: `
         *   *   *
        * * * * * *
         * * * * *
        * * * * * *
          ||||||
          ||||||
                  //||||\\\\
                      `
                    }
                  }
                },
                erica: {
                  name: "Erica",
                  rank: "genus",
                  commonName: "Heath / Bell Heather",
                  description: "A large genus of ericaceous shrubs, with delicate hanging bell-shaped flowers and needle-like leaves.",
                  children: {
                    tetralix: {
                      name: "Erica tetralix",
                      rank: "species",
                      commonName: "Cross-leaved Heath",
                      gaelicName: "Fraoch-frangach (French Heather / Bog Heather)",
                      geologicalEra: "Miocene, ~6 Mya",
                      evolutionaryMilestone: "Leaves arranged in neat whorls of four (forming a cross) covered in sticky glandular hairs to catch moisture.",
                      description: "A beautiful bog-dwelling shrub with delicate pink, nodding egg-shaped bell flowers and greyish needle leaves.",
                      habitat: "Wet bogs, damp blanket mires, soggy heathlands, and acidic pool margins.",
                      lore: "Unlike dry-loving Ling Heather, Cross-leaved Heath grows in deep, squelching Scottish bogs. It was historically used to weave small scrubbing brushes and was known for warning wanderers that they were entering dangerously wet peatland.",
                      status: "Common in wet peatlands.",
                      asciiArt: `
          _(_)(_)_
         (_)(_)(_)
          /||||\\
         / |||| \\
          /||||\\
           ||||
           ||||
                      `
                    }
                  }
                }
              }
            },
            rosaceae: {
              name: "Rosaceae",
              rank: "family",
              commonName: "Rose Family",
              geologicalEra: "Cretaceous, ~85 Mya",
              evolutionaryMilestone: "Evolution of showy flowers with multi-petaled petals and copious nectar to attract a diverse range of generalist insect pollinators.",
              description: "A massive, diverse family of trees, shrubs, and herbs, including many of humanity's most prized fruits (apples, berries, almonds) and garden roses.",
              children: {
                sorbus: {
                  name: "Sorbus",
                  rank: "genus",
                  commonName: "Whitebeam, Rowan & Allies",
                  description: "Deciduous trees with simple or pinnate leaves and clusters of white flowers followed by bright red berries.",
                  children: {
                    aucuparia: {
                      name: "Sorbus aucuparia",
                      rank: "species",
                      commonName: "Rowan / Mountain Ash / Lady of the Forest",
                      gaelicName: "Caorunn (The Protection Tree)",
                      geologicalEra: "Miocene, ~15 Mya",
                      evolutionaryMilestone: "Feathery compound leaves and bright red anthocyanin-rich berries designed for consumption by wintering thrushes (redwings/fieldfares) for seed dispersal.",
                      description: "A stunning, small deciduous tree with feathery leaves, fragrant white spring blossoms, and heavy, dropping clusters of scarlet berries in autumn.",
                      habitat: "Rugged cliffs, streamsides, old pine forests, and rocky Highland slopes up to high altitudes.",
                      lore: "Rowan is the most sacred tree in Scottish folklore. Planted near every Highland home, blackhouse, and cowshed, its fiery red berries were believed to represent the fire of life, warding off witches, evil spirits, and bad luck. Its wood was never cut except for sacred purposes, such as making butter churns to protect the dairy from curses.",
                      status: "Common and highly cherished across Scotland.",
                      asciiArt: `
         o   o   o
         \\|/ \\|/ \\|/
          |   |   |
         -+---+---+-
             |||
             |||
             |||
            /|||\\
                      `
                    }
                  }
                },
                rubus: {
                  name: "Rubus",
                  rank: "genus",
                  commonName: "Bramble & Raspberry Genus",
                  description: "A vast genus of woody brambles, known for their compound leaves, prickly stems, and delicious aggregate fruits.",
                  children: {
                    chamaemorus: {
                      name: "Rubus chamaemorus",
                      rank: "species",
                      commonName: "Cloudberry / Knotberry / Avery",
                      gaelicName: "Oireag (The Highland Amber)",
                      geologicalEra: "Glacial Relic, ~3 Mya",
                      evolutionaryMilestone: "Dwarf creeping growth habit and cold-hardiness down to -40C, enabling survival of post-glacial tundra conditions.",
                      description: "A low, herbaceous alpine bramble with single white flowers and unique amber-colored aggregate berries.",
                      habitat: "High altitude blanket bogs, peat mosses, and mountaintop plateaus.",
                      lore: "Cloudberry is a glacial relic that followed the retreating ice sheets north. It grows only in high, cold Highland bogs (like the Cairngorms). The juicy, amber berries taste of a unique mix of honey and apple. Historically, they were highly prized by highlanders as a valuable source of Vitamin C in the high mountains.",
                      status: "Uncommon; restricted to pristine, cold montane peatlands.",
                      asciiArt: `
            (\\./)
           (( Q ))
            (/\\/)
            //|\\\\
           // || \\\\
             |||
             |||
                      `
                    }
                  }
                }
              }
            },
            campanulaceae: {
              name: "Campanulaceae",
              rank: "family",
              commonName: "Bellflower Family",
              geologicalEra: "Cretaceous, ~80 Mya",
              evolutionaryMilestone: "Evolution of nodding, pendulous bell flowers that shelter pollen and nectar from heavy Highland rains.",
              description: "A family of herbaceous plants with blue, bell-shaped flowers and milky sap.",
              children: {
                campanula: {
                  name: "Campanula",
                  rank: "genus",
                  commonName: "Bellflower",
                  description: "Delicate plants characterized by blue or purple campanulate (bell-shaped) flowers and simple alternate leaves.",
                  children: {
                    rotundifolia: {
                      name: "Campanula rotundifolia",
                      rank: "species",
                      commonName: "Harebell / Bluebell of Scotland",
                      gaelicName: "Clag-gorm (The Blue Bell)",
                      geologicalEra: "Pleistocene, ~1.5 Mya",
                      evolutionaryMilestone: "Extremely thin, flexible stems that yield to wind, preventing mechanical snapping during severe Atlantic gales.",
                      description: "A delicate, enchanting plant with nodding, paper-thin sky-blue bells growing on hair-fine, resilient stems.",
                      habitat: "Dry pastures, rocky crevices, roadside banks, and dry Highland hill turf.",
                      lore: "Known in Scotland as the true 'Bluebell' (unlike the English woodland bluebell), it is often called 'Witches' Thimbles' or 'Goblin's Gloves' in folklore. Shaking the bell was said to summon malevolent fairies or ring a death knell, and picking one was believed to invite fairy curses.",
                      status: "Widespread, but declining in heavily grazed areas.",
                      asciiArt: `
           (( ))
          ((   ))
           \\_=_/
            | |
            | |
          --+---+--
            | |
                      `
                    }
                  }
                }
              }
            },
            asteraceae: {
              name: "Asteraceae",
              rank: "family",
              commonName: "Daisy / Thistle Family",
              geologicalEra: "Cretaceous, ~80 Mya",
              evolutionaryMilestone: "The composite head (pseudanthium), grouping hundreds of microscopic florets together to look like a single massive flower.",
              description: "One of the most successful plant families, featuring daisy-like composite flowers or flower heads packed with defensive spiny bracts, as seen in thistles.",
              children: {
                cirsium: {
                  name: "Cirsium",
                  rank: "genus",
                  commonName: "Plume Thistle",
                  description: "Spiny, erect herbs with purple-pink flower heads encased in spherical, intensely prickly arrays of green bracts.",
                  children: {
                    vulgare: {
                      name: "Cirsium vulgare",
                      rank: "species",
                      commonName: "Spear Thistle",
                      gaelicName: "Cluaran (The Noble Prickle)",
                      geologicalEra: "Neogene, ~5 Mya",
                      evolutionaryMilestone: "Extremely sharp, spear-tipped leaves and spiny stem-wings providing absolute physical defense against heavy grazing.",
                      description: "A tall, majestic, and intensely spiky plant with pale purple flower heads. It is the historic floral emblem and national symbol of Scotland.",
                      habitat: "Pastures, roadsides, forest clearings, waste ground, and sand dunes.",
                      lore: "During the reign of Alexander III, Norse invaders under King Haakon attempted a midnight sneak attack on the Scots at Largs. To maintain silence, the Norsemen crept barefoot. One unfortunate warrior stepped squarely on a sharp Spear Thistle, crying out in agony. The alarm was raised, the Scots rallied, routed the Norsemen, and adopted the thistle as their noble protector.",
                      status: "Very common and aggressive native thistle.",
                      asciiArt: `
         _  _  _
        (_)(_)(_)
        \\  ||  /
        {==||==}
         \\ || /
         _\\||/_
        /  ||  \\
       /   ||   \\
                      `
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
};
