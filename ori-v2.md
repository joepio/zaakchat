# ORI V2 API Ontwerp

_Door Joep Meindertsma, 21 januari 2026_

In dit document wil ik uiteenzetten hoe een vernieuwde, volgende generatie API voor Open RaadsInformatie (ORI) er uit zou kunnen zien.

Al ongeveer 8 jaar ben ik betrokken bij het ORI project (dan bedoel ik vooral [de implementatie](https://github.com/openstate/open-raadsinformatie/issues) en de [zoekinterface](https://github.com/ontola/openbesluitvorming/)).
In die tijd is het opgeschaald van 5 gemeenten naar meer dan 300+ gemeenten, provincies en waterschappen.
Er zitten nu al meer dan 5.000.000 documenten in. Volgens mij is het daarmee een van de grootste en meest succesvolle open data projecten van Nederland.

De huidige ORI implementatie werkt door de verschillende API's van leveranciers om te zetten naar een centraal systeem.
Al enkele jaren willen we toe naar een echte standaard: een enkele API die door de verschillende RIS leveranciers wordt gebruikt om informatie mee uit te wisselen.
Met de VNG ORI API (deze repository) zouden we zowel gestructureerde data (zoals agendapunten) en documenten (zoals raadsstukken) op een uniforme manier kunnen aanbieden en doorzoekbaar maken.

Maar met de komst van de WOO is er de mogelijkheid om grotere ambities te hebben, en ook andere (WOO) informatiecategorieen toe te voegen.
In dit document wil ik uiteenzetten hoe een vernieuwde, volgende generatie API voor Open RaadsInformatie er uit zou kunnen zien.

## Huidige situatie

### Per applicatie / domein een eigen API

Nu krijgen specifieke WOO informatiecategorieen (ORI, WOZ, etc) allemaal een eigen API. Dit is een begrijpelijke keuze, want zo werkt data hergebruik in de meeste situaties: iedere dienst heeft een eigen API. Dit paradigma functioneert, maar heeft tekortkomingen als de schaal en complexiteit erg groot worden:

- **Hergebruik datamodellen is beperkt**. Sommige datamodellen (zoals Persoon, of Document) komen terug in diverse applicaties. Het is dan verspilling van tijd om per applicatie opnieuw discussie te voeren over welke velden er in moeten, en hoe ze moeten heten. Maar dit soort discussies krijgen we nu wel (zoals ik laatst zag in een MijnGesprekken meeting). Dit is te voorkomen met herbruikbare datamodellen. Helaas gaat dit herbruiken van datamodellen erg lastig als je ze koppelt aan de OpenAPISpec.
- **Inconsistentie van patronen en protocol**. Per API worden er andere keuzes gemaakt over zaken als filtering, querying, paginering, response codes en meer. Dat betekent effectief dat het wiel heel veel keren opnieuw wordt uitgevonden, waardoor er per API andere code moet worden geschreven. Dat levert werk op aan zowel de kant van de producer (leverancier) als consumer (hergebruiker).
- **Geen gestandaardiseerd protocol**. De API's worden weliswaar uitgedrukt in een machineleesbaar formaat (OpenAPI spec), maar alsnog is het protocol zelf niet gestandaardiseerd. Als ontwikkelaar zou je dus per API weer moeten werken om alles opnieuw in te bouwen

### Per datamodel een eigen endpoint

Binnen een API specificatie krijgt ieder model (bijv. Document, Taak, Zaak) een eigen endpoint (/taken). Hier is het schrijven (command, POST) en het uitlezen (query, GET) sterk aan elkaar gekoppeld en uitgewerkt per datamodel. Wederom begrijpelijk, maar bij grotere en meer complexe systemen is dit niet meer optimaal:

- **Complexe API specs**. Met ieder extra datamodel groeit je specificatie. De huidige ORI API is duizenden regels groot. Dit is voor leveranciers een gigantische klus om te implementeren. Dat vertraagt adoptie van je standaard, en maakt daarmee het hele project duurder en langer. Dit lijkt me op zichzelf al genoeg reden om deze aanpak in twijfel te trekken.
- **Versiebeheer ingewikkeld**. Als je datamodellen aan je versiebeheer van je API / Endpoint zijn gekoppeld, wordt het lastig om nieuwe datamodellen te delen. Dat betekent een versie bump voor beiden. Wanneer een Producer (die op v1.2 draait) een datamodel wil aanleveren aan een Consumer (die op v1.1 draait), maar de Consumer dit datamodel nog niet heeft, dan loopt het geheel stuk. De Consumer kan deze data niet krijgen. Er ontstaan gaten in de data. De oplossing voor dit probleem: trek datamodel en API los van elkaar.

### Informatiemodellen en datamodellen als losse concepten

Op dit moment worden informatiemodellen ontworpen in UML, mermaid, of andere tooling. Met tools als Imvertor worden APIs (OpenAPISpec bestanden) soms zelfs gegenereerd uit deze modellen. Het hebben van heldere en passende modellen is essentieel en ik zie daar absoluut de noodzaak van in. Maar wanneer de informatiemodellen los staan van de datamodellen (in de API) spec, kan dit tot problemen leiden:

- **Datamodellen zijn niet herbruikbaar**. De informatie model bestanden zijn slechts zeer beperkt machine leesbaar / converteerbaar. Mogelijk zijn ze niet eens opvraagbaar over het web op gestandaardiseerde wijze. Dit beperkt in hoevere anderen ze kunnen gebruiken.
- **Verschil tussen datamodel en informatiemodel leidt tot verwarring**. Als informatiemodel en datamodel aparte concepten zijn, gaan daar verschillen tussen ontstaan. Dat maakt het voor hergebruikers ingewikkelder om te begrijpen en kan leiden tot onnodige verwarring.
- **Leidt tot onlogische modellen**. Een informatiemodel moet is nooit een neutrale beschrijving van de werkelijkheid - het is altijd een specifieke invulling die met een bepaalde bril logisch lijkt. Welke bril kies je dan? In mijn optiek, moet dat de bril zijn van degene die de data moet gaan gebruiken, en dat is in dit geval vooral de applicatiebouwer. De functionele eisen moeten bepalen in welke vorm je data staat. Als deze wensen niet in het informatiemodel worden meegenomen, kan het informatiemodel beperkend en problematisch zijn wanneer de applicatie gaat worden gebouwd. Ik zie dit helaas nu al (bijna) gebeuren bij het MijnGesprekken ontwerp. Dit kan onnodige vertraging opleveren bij implementatie.

### Objecten zijn persisted, wijzigingen (events) niet

We slaan de huidige versie op van documenten. Als we iets wijzigen, wordt deze wijziging toegepast, maar wordt de wijziging zelf niet _opgeslagen_. De wijziging bestaat alleen als tijdelijk bericht tussen een gebruiker en een database.
Dat maakt synchroniseren erg lastig. De bestaande REST APIs hebben geen mogelijkheden om betrouwbaar en eenvoudig (alle) gegevens van een producer naar een consumer te krijgen. Dat heeft enkele problemen:

- **Fouten en gaten in data**. Per object moet er een POST worden gemaakt, naar een specifiek endpoint, waar vervolgens om allerlei redenen fouten kunnen ontstaan (netwerk fouten, data fouten). Als deze fouten voorkomen, is het de verantwoordelijkheid van de Producer om het later nog eens te proberen. Dit betekent dat de Producer een lijst moet bijhouden van items die er mis zijn gegaan, of dat hij bij moet houden welke informatie de Consumer heeft. Dat is een gigantisch complexe en foutgevoelige taak. En deze verantwoordelijkheid leggen we effectief bij iedere leverancier - een onredelijk grote taak, waar ze waarschijnlijk ofwel fouten in gaan maken (wat leidt tot dataverlies) ofwel überhaupt niet willen inbouwen (waardoor adoptie laag is en de standaard niet wordt gebruikt)
- **Vendor lock-in / lage dataportabiliteit**. Data van systeem A naar B krijgen kan soms zeer kostbaar zijn. Leveranciers werken soms actief tegen, of vragen veel te veel geld als data moet worden overgezet.
- **Geen betrouwbaar verwijderen mogelijk**. Soms worden per ongeluk items gepubliceerd met persoonsgegevens er in. Ik ben dit helaas vaak tegengekomen bij Open Raadsinformatie (fouten in anonimisatie / lak software, slordige griffie medewerkers). We hebben geen automatische manier om dit soort fouten te herstellen. Als een medewerker braaf de data publiceert uit een bronsysteem, is dat geen garantie dat dit ook wordt verwijderd bij de Consumer.
- **Geen realtime applicaties**. In veel domeinen is real-time geen eis, maar in bijvoorbeeld een chat applicatie (zoals MijnGesprekken) is dat wel het geval. Real-time kan zeer lastig zijn en veel complexiteit met zich mee brengen. Als je dit niet vroeg (en generiek genoeg) oplost, gaat het veel geld kosten en wederom adoptie vertragen.

### Beperkte query mogelijkheden in APIs

Doordat we per model (zoals Zaak, Document, etc) een endpoint ontwerpen, kunnen er per model andere query mogelijkheden zijn. Dit brengt niet alleen veel complexiteit met zich mee (wat adoptie vertraagt), maar is ook inherent beperkend voor hergebruikers - het zegt namelijk dat er ook veel query mogelijkheden niet beschikbaar zijn.

Als hergebruiker (bijvoorbeeld als bouwer van de MijnZaken app) heb je soms zeer specifieke zoek (query)wensen. Misschien wil je alle zaken hebben van een bepaalde gebruiker (een filter op een veld), en misschien wil je die wel sorteren op datum (een sort attribuut), of misschien wil je wel alleen de zaken van die gebruiker zien waarbij de status op "open" staat. Misschien wil je ook wel laten zien hoeveel documenten er dan beschikbaar zijn.

Nu lossen we dit soort problemen op door per endpoint, per datamodel, heel veel discussie te voeren en heel veel attributen toe te voegen. Daardoor wordt je API specificatie zeer groot (wat nu het geval is), waardoor implementatie (en dus adoptie) stroef loopt en foutgevoelig is. En tenslotte ga je usecases over het hoofd zien, waardoor je toch altijd achter loopt - en dus beperkte query mogelijkheden houdt.

Zo zijn er nu in de APIs nog geen manieren om full-text-search te doen - een functionaliteit die in veel usecases zeer waardevol is. 

## Voorgestelde situatie

### Alle VNG Datamodellen beschikbaar stellen via JSON-Schema

In plaats van je informatiemodellen om te zetten naar een OpenAPI specificatie, zetten we bestaande modellen om naar [JSON Schema](https://json-schema.org/). Dit zijn JSON objecten die beschrijven welke vorm je data moet hebben. Dit is naar mijn mening de juiste standaard om hiervoor te kiezen:

- **Blijft OAS Compatible**. Deze standaard kan eveneens gewoon gebruikt worden in OpenAPI documenten.
- **Veel libraries beschikbaar**. Voor de meeste programmeertalen zijn hoogwaardige JSON Schema implementaties te vinden. Dit maakt het goedkoop om te adopteren.
- **JSON is de standaard**. Omdat praktisch alle moderne applicaties al werken met JSON, is dit gewoon precies wat ontwikkelaars verwachten.

Ik stel voor dat we 1 Github respository krijgen voor JSON Schema datamodellen. In deze repository is altijd de laatste versie te vinden van alle datamodellen. We zetten vervolgens deze JSON Schema bestanden beschikbaar in een simpele, doeltreffende, gebruiksvriendelijke webinterface: VNG Datamodellen. Dit online beschikbaar stellen biedt andere belangrijke voordelen:

- **Geautomatiseerd testen**. Doordat met deze opzet de JSON Schema modellen als HTTP URL opvraagbaar zijn, kunnen leveranciers in hun ontwikkelstraat (CI/CD) per model testen of hun applicatie hieraan voldoet. Je kunt ook eenvoudig JSON Schema compatible fuzzing doen.
- **Gegenereerde formulieren**. Een JSON Schema document kan worden gebruikt om formulieren mee te bouwen die gegarandeerd voldoen aan de vorm van de data. Dit maakt het ontwikkelen van formulieren per model geheel gratis! Dit betekent dat leveranciers in hun applicaties aan hun gebruikers direct VNG-specifieke datamodellen kunnen presenteren, zodat data veel beter gestandaardiseerd en dus herbruikbaar zal zijn. 

Doordat we de datamodellen zien als losstaand van de API, kunnen discussies over welke velden welke namen krijgen, welke velden verplicht zijn etc. ook los worden gekoppeld van hoe de API zelf functioneert.

### Alle wijzigingen als Events opslaan

In plaats van alleen de _huidige staat_ van een object op te slaan, kunnen we de _wijzigingen_ van een object opslaan. Dit heet ook wel Event Sourcing. Deze strategie heeft enkele voordelen:

- **Eenvoudige synchronisatie**. Een hergebruiker die geinteresseerd is in de volledige, up-to-date staat, kan simpelweg subscriben op events.
- **Real-time updates**. Die events kunnen (bijvoorbeeld via HTTP SSE) over een enkele verbinding worden verzonden.
- **Auditability**. Door alle wijzigingen op te slaan, kunnen we inzicht geven in wie wat wanneer heeft gewijzigd.
- **Versioning / history**. Iedere oude versie van een document is op te bouwen door de events tot op dat moment af te spelen.

### Generieke APIs, 3 endpoints

In plaats van dat we per domein een eigen API hebben, en per datamodel een eigen endpoint, hebben we een enkele API die alle functionaliteiten op generieke, standaard wijze aanbiedt. We gebruiken bestaande standaarden waar mogelijk, en volgen industry best practices. Doordat we JSON-Schema gebruiken voor de datamodellen, hoeven we niet meer model-specifieke informatie te stoppen in onze API modellen.

De exacte vorm van deze generieke API is een belangrijke discussie. Over de invulling heb ik wel al gedachten. Mijn opzet is als volgt:

- **Event Endpoint**. We doen Event Sourcing, middels de CloudEvents standaard. Producers (leveranciers) maken "events" aan, waarin wijzigingen en gebeurtenissen beschreven staan. Deze worden opgeslagen en kunnen worden herbruikt. Consumers / hergebruikers kunnen deze CloudEvents ophalen. Deze CloudEvents beschrijven veranderingen in data. Dankzij dit paradigma is het mogelijk om systemen foutloos in sync te houden, real-time updates over de lijn te sturen, en dit alles te realiseren op een manier die veel minder tijd en geld kost dan diverse endpoints te onderhouden. Voor een concrete invulling hiervan, zie mijn [JSONCommit](https://github.com/vng-realisatie/zaakchat/blob/main/JSONCOMMIT.md) specificatie en werkende demo implementatie hiervan op [ZaakChat.nl](https://zaakchat.nl). Doordat Events
- **Resource Endpoint**. De huidige staat van iedere resource is individueel opvraagbaar bij de bron, middels een URL, als JSON object. De vormen van deze resources voldoen aan de JSON Schemas die ik hierboven heb beschreven. Dit is wat mij betreft de kernkracht van linked data, en de reden waarom het world wide web zo succesvol geworden is. Iedere resource heeft een URL als identifier, en deze kan je gewoon opvragen (mits je de juiste rechten hebt). Data blijft bij de bron, blijft
- **Query Endpoint**. Een enkele (mogelijk ElasticSearch-compatible) zoekinterface met krachtige query mogelijkheden. Dit biedt de mogelijkheid om simpele vragen te beantwoorden zoals: "geef me alle documenten", maar ook complexe, zoals "geef me alle documenten van pieter, gesorteerd op datum, waar de bestandsgrootte meer dan 1MB is". Dit is het type zoektechnologie wat ook gebruikt wordt in webwinkels, of platforms zoals [OpenBesluitvorming](https://openbesluitvorming.nl/).
