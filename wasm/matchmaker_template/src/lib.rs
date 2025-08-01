use std::collections::VecDeque;

use anoma_vm_env::matchmaker_prelude::intent::{Intent, IntentTransfers};
use anoma_vm_env::matchmaker_prelude::key::ed25519::Signed;
use anoma_vm_env::matchmaker_prelude::*;
use petgraph::graph::{node_index, DiGraph, NodeIndex};
use petgraph::visit::{depth_first_search, Control, DfsEvent};
use petgraph::Graph;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IntentNode {
    id: Vec<u8>,
    intent: Signed<Intent>,
}

#[matchmaker]
fn add_intent(graph_bytes: Vec<u8>, id: Vec<u8>, data: Vec<u8>) -> bool {
    let intent = decode_intent_data(&data);
    let mut graph = decode_graph(graph_bytes);
    log_string(format!("trying to match intent: {:#?}", intent));
    add_node(&mut graph, id, intent);
    find_match_and_remove_node(&mut graph);
    update_graph_data(&graph);
    true
}

fn create_transfer(
    from_node: &IntentNode,
    to_node: &IntentNode,
) -> token::Transfer {
    token::Transfer {
        source: from_node.intent.data.addr.clone(),
        target: to_node.intent.data.addr.clone(),
        token: to_node.intent.data.token_buy.clone(),
        amount: to_node.intent.data.amount_buy,
    }
}

fn send_tx(tx_data: IntentTransfers) {
    let tx_data_bytes = tx_data.try_to_vec().unwrap();
    send_match(tx_data_bytes);
}

fn decode_intent_data(bytes: &[u8]) -> Signed<Intent> {
    Signed::<Intent>::try_from_slice(bytes).unwrap()
}

fn decode_graph(bytes: Vec<u8>) -> DiGraph<IntentNode, Address> {
    if bytes.is_empty() {
        Graph::new()
    } else {
        serde_json::from_slice(&bytes[..]).expect("error in json format")
    }
}

fn update_graph_data(graph: &DiGraph<IntentNode, Address>) {
    update_data(serde_json::to_vec(graph).unwrap());
}

fn find_to_update_node(
    graph: &DiGraph<IntentNode, Address>,
    new_node: &IntentNode,
) -> (Vec<NodeIndex>, Vec<NodeIndex>) {
    let start = node_index(0);
    let mut connect_sell = Vec::new();
    let mut connect_buy = Vec::new();
    depth_first_search(graph, Some(start), |event| {
        if let DfsEvent::Discover(index, _time) = event {
            let current_node = &graph[index];
            if new_node.intent.data.token_sell
                == current_node.intent.data.token_buy
                && new_node.intent.data.amount_sell
                    == current_node.intent.data.amount_buy
            {
                connect_sell.push(index);
            } else if new_node.intent.data.token_buy
                == current_node.intent.data.token_sell
                && new_node.intent.data.amount_buy
                    == current_node.intent.data.amount_sell
            {
                connect_buy.push(index);
            }
        }
        Control::<()>::Continue
    });
    (connect_sell, connect_buy)
}

fn add_node(
    graph: &mut DiGraph<IntentNode, Address>,
    id: Vec<u8>,
    intent: Signed<Intent>,
) {
    let new_node = IntentNode { id, intent };
    let new_node_index = graph.add_node(new_node.clone());
    let (connect_sell, connect_buy) = find_to_update_node(&graph, &new_node);
    let sell_edge = new_node.intent.data.token_sell;
    let buy_edge = new_node.intent.data.token_buy;
    for node_index in connect_sell {
        graph.update_edge(new_node_index, node_index, sell_edge.clone());
    }
    for node_index in connect_buy {
        graph.update_edge(node_index, new_node_index, buy_edge.clone());
    }
}

fn create_and_send_tx_data(
    graph: &DiGraph<IntentNode, Address>,
    cycle_intents: Vec<NodeIndex>,
) {
    log_string(format!(
        "found match; creating tx with {:?} nodes",
        cycle_intents.len()
    ));
    let cycle_intents = sort_cycle(graph, cycle_intents);
    let mut cycle_intents_iter = cycle_intents.into_iter();
    let first_node = cycle_intents_iter.next().map(|i| &graph[i]).unwrap();
    let mut tx_data = IntentTransfers::empty();
    let last_node =
        cycle_intents_iter.fold(first_node, |prev_node, intent_index| {
            let node = &graph[intent_index];
            tx_data.transfers.insert(create_transfer(node, prev_node));
            tx_data
                .intents
                .insert(node.intent.data.addr.clone(), node.intent.clone());
            &node
        });
    tx_data
        .transfers
        .insert(create_transfer(first_node, last_node));
    tx_data.intents.insert(
        first_node.intent.data.addr.clone(),
        first_node.intent.clone(),
    );
    send_tx(tx_data)
}

// The cycle returned by tarjan_scc only contains the node_index in an arbitrary
// order without edges. we must reorder them to craft the transfer
fn sort_cycle(
    graph: &DiGraph<IntentNode, Address>,
    cycle_intents: Vec<NodeIndex>,
) -> Vec<NodeIndex> {
    let mut cycle_ordered = Vec::new();
    let mut cycle_intents = VecDeque::from(cycle_intents);
    let mut to_connect_node = cycle_intents.pop_front().unwrap();
    cycle_ordered.push(to_connect_node);
    while !cycle_intents.is_empty() {
        let pop_node = cycle_intents.pop_front().unwrap();
        if graph.contains_edge(to_connect_node, pop_node) {
            cycle_ordered.push(pop_node);
            to_connect_node = pop_node;
        } else {
            cycle_intents.push_back(pop_node);
        }
    }
    cycle_ordered.reverse();
    cycle_ordered
}

fn find_match_and_send_tx(
    graph: &DiGraph<IntentNode, Address>,
) -> Vec<NodeIndex> {
    let mut to_remove_nodes = Vec::new();
    for cycle_intents in petgraph::algo::tarjan_scc(&graph) {
        // a node is a cycle with itself
        if cycle_intents.len() > 1 {
            to_remove_nodes.extend(&cycle_intents);
            create_and_send_tx_data(graph, cycle_intents);
        }
    }
    to_remove_nodes
}

fn find_match_and_remove_node(graph: &mut DiGraph<IntentNode, Address>) {
    let mut to_remove_nodes = find_match_and_send_tx(&graph);
    // Must be sorted in reverse order because it removes the node by index
    // otherwise it would not remove the correct node
    to_remove_nodes.sort_by(|a, b| b.cmp(a));
    to_remove_nodes.into_iter().for_each(|i| {
        graph.remove_node(i);
    });
}
