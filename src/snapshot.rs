// The goal here is to create a snapshot using vector clocks
// to do that we need to find the latest consistent cut 

/* 
• A global snapshot captures
    1. 2. The local states of each process (e.g., program variables), and
    The state of each communication channel
*/

// when we want to take a snapshot we need all the other instances to send their clocks

// warning : if the network is not completely connected we need to find another way to get 
// the clocks of the other instances


// 1. lancer la demande de snapshot
// 2. recuperer les horloges des autres instances
// 3. trouver la coupe consistante
// 4. enregistrer l'état local correspondant aux horloges de la coupe
// 5. enregistrer l'état des canaux de communication


// other sources used : https://www.cs.princeton.edu/courses/archive/fall18/cos418/docs/L4-vc.pdf

// note : théoriquement on n'a pas besoin de redemander les horloges des autres instances
// car pour l'instant on est dans le cas d'un réseau complet et qui communique dès qu'une action est faite
// donc les horloges qu'on maintient sont relativmeent à jour

pub fn start_snapshot() {
    // 1. Send a snapshot request to all peers
    crate::network::send_message_to_all(
        NetworkMessageCode::SnapshotRequest,
        MessageInfo::None,
        None,
    );

    // 2. Wait for snapshot responses from all peers
    

}


