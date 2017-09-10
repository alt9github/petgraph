
use serde::de::Error;

use std::marker::PhantomData;

use prelude::*;

use EdgeType;
use graph::Node;
use graph::{IndexType, Edge};
use serde_utils::MappedSequenceVisitor;
use serde_utils::CollectSeqWithLength;
use serde_utils::{IntoSerializable, FromDeserialized};

use super::{NodeIndex, EdgeIndex};
use serde::{Serialize, Serializer, Deserialize, Deserializer};

// Serialization representation for Graph
// Keep in sync with deserialization and StableGraph
#[derive(Serialize)]
#[serde(rename = "Graph")]
#[serde(bound(serialize = "N: Serialize, E: Serialize, Ix: IndexType + Serialize"))]
pub struct SerGraph<'a, N: 'a, E: 'a, Ix: 'a + IndexType> {
    #[serde(serialize_with="ser_graph_nodes")]
    nodes: &'a [Node<N, Ix>],
    node_holes: &'a [NodeIndex<Ix>],
    edge_property: EdgeProperty,
    #[serde(serialize_with="ser_graph_edges")]
    edges: &'a [Edge<E, Ix>],
}

// Deserialization representation for Graph
// Keep in sync with serialization and StableGraph
#[derive(Deserialize)]
#[serde(rename = "Graph")]
#[serde(bound(deserialize = "N: Deserialize<'de>, E: Deserialize<'de>, Ix: IndexType + Deserialize<'de>"))]
pub struct DeserGraph<N, E, Ix> {
    #[serde(deserialize_with="deser_graph_nodes")]
    nodes: Vec<Node<N, Ix>>,
    #[serde(deserialize_with="deser_graph_node_holes")]
    #[allow(unused)]
    #[serde(default="Vec::new")]
    node_holes: Vec<NodeIndex<Ix>>,
    edge_property: EdgeProperty,
    #[serde(deserialize_with="deser_graph_edges")]
    edges: Vec<Edge<E, Ix>>,
}


impl<Ix> Serialize for NodeIndex<Ix>
    where Ix: IndexType + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        self.0.serialize(serializer)
    }
}

impl<'de, Ix> Deserialize<'de> for NodeIndex<Ix>
    where Ix: IndexType + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        Ok(NodeIndex(Ix::deserialize(deserializer)?))
    }
}

impl<Ix> Serialize for EdgeIndex<Ix>
    where Ix: IndexType + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        self.0.serialize(serializer)
    }
}

impl<'de, Ix> Deserialize<'de> for EdgeIndex<Ix>
    where Ix: IndexType + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        Ok(EdgeIndex(Ix::deserialize(deserializer)?))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all="lowercase")]
#[derive(Debug)]
pub enum EdgeProperty {
    Undirected,
    Directed,
}

impl EdgeProperty {
    pub fn is_directed(&self) -> bool {
        match *self {
            EdgeProperty::Directed => true,
            EdgeProperty::Undirected => false,
        }
    }
}

impl<Ty> From<PhantomData<Ty>> for EdgeProperty where Ty: EdgeType {
    fn from(_: PhantomData<Ty>) -> Self {
        if Ty::is_directed() {
            EdgeProperty::Directed
        } else {
            EdgeProperty::Undirected
        }
    }
}


fn ser_graph_nodes<S, N, Ix>(nodes: &&[Node<N, Ix>], serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer,
          N: Serialize,
          Ix: Serialize + IndexType,
{
    serializer.collect_seq_exact(
        nodes.iter()
             .map(|node| &node.weight))
}

fn ser_graph_edges<S, E, Ix>(edges: &&[Edge<E, Ix>], serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer,
          E: Serialize,
          Ix: Serialize + IndexType,
{
    serializer.collect_seq_exact(
        edges.iter()
            .map(|edge| Some((
               edge.source(),
               edge.target(),
               &edge.weight,
            ))))
}


fn deser_graph_nodes<'de, D, N, Ix>(deserializer: D) -> Result<Vec<Node<N, Ix>>, D::Error>
    where D: Deserializer<'de>,
          N: Deserialize<'de>,
          Ix: IndexType + Deserialize<'de>,
{
    deserializer.deserialize_seq(MappedSequenceVisitor::new(|n|
        Ok(Node {
            weight: n,
            next: [EdgeIndex::end(); 2],
        })
    ))
}

fn deser_graph_node_holes<'de, D, Ix>(deserializer: D) -> Result<Vec<NodeIndex<Ix>>, D::Error>
    where D: Deserializer<'de>,
          Ix: IndexType + Deserialize<'de>,
{
    deserializer.deserialize_seq(MappedSequenceVisitor::<NodeIndex<Ix>, NodeIndex<Ix>, _>::new(|_| {
        Err("Graph can not have holes in the node set, found non-empty node_holes")
    }))
}

fn deser_graph_edges<'de, D, N, Ix>(deserializer: D) -> Result<Vec<Edge<N, Ix>>, D::Error>
    where D: Deserializer<'de>,
          N: Deserialize<'de>,
          Ix: IndexType + Deserialize<'de>,
{
    deserializer.deserialize_seq(MappedSequenceVisitor::<Option<(NodeIndex<Ix>, NodeIndex<Ix>, N)>, _, _>::new(|x|
        if let Some((i, j, w)) = x {
            Ok(Edge {
                weight: w,
                node: [i, j],
                next: [EdgeIndex::end(); 2],
            })
        } else {
            Err("Graph can not have holes in the edge set, found None, expected edge")
        }
    ))
}

impl<'a, N, E, Ty, Ix> IntoSerializable for &'a Graph<N, E, Ty, Ix>
    where Ix: IndexType,
          Ty: EdgeType,
{
    type Output = SerGraph<'a, N, E, Ix>;
    fn into_serializable(self) -> Self::Output {
        SerGraph {
            nodes: &self.nodes,
            node_holes: &[],
            edges: &self.edges,
            edge_property: if self.is_directed() {
                EdgeProperty::Directed
            } else {
                EdgeProperty::Undirected
            },
        }
    }
}

impl<'a, N, E, Ty, Ix> FromDeserialized for Graph<N, E, Ty, Ix>
    where Ix: IndexType,
          Ty: EdgeType,
{
    type Input = DeserGraph<N, E, Ix>;
    fn from_deserialized<E2>(input: Self::Input) -> Result<Self, E2>
        where E2: Error
    {
        if input.edge_property.is_directed() != Ty::is_directed() {
            return Err(E2::custom(format_args!("graph edge property mismatch, \
                                               expected {:?}, found {:?}",
                                               EdgeProperty::from(PhantomData::<Ty>),
            input.edge_property)));
        }
        assemble_graph(input.nodes, input.edges)
    }
}

impl<N, E, Ty, Ix> Serialize for Graph<N, E, Ty, Ix>
    where Ty: EdgeType,
          Ix: IndexType + Serialize,
          N: Serialize,
          E: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        self.into_serializable().serialize(serializer)
    }
}


pub fn invalid_node_err<E>(node_index: usize, len: usize) -> E where E: Error {
    E::custom(format_args!("invalid value: node index `{}` does not exist in graph \
                           with node bound {}",
                           node_index, len))
}

pub fn invalid_length_err<Ix, E>(node_or_edge: &str, len: usize) -> E
    where E: Error, Ix: IndexType
{
    E::custom(format_args!("invalid size: graph {} count {} exceeds index type maximum {}",
                           node_or_edge, len, <Ix as IndexType>::max().index()))
}

fn assemble_graph<N, E, Ty, Ix, E2>(nodes: Vec<Node<N, Ix>>, edges: Vec<Edge<E, Ix>>) -> Result<Graph<N, E, Ty, Ix>, E2>
    where E2: Error,
          Ty: EdgeType,
          Ix: IndexType,
{
    if nodes.len() >= <Ix as IndexType>::max().index() {
        Err(invalid_length_err::<Ix, _>("node", nodes.len()))?
    }

    if edges.len() >= <Ix as IndexType>::max().index() {
        Err(invalid_length_err::<Ix, _>("edge", edges.len()))?
    }

    let mut gr = Graph {
        nodes: nodes,
        edges: edges,
        ty: PhantomData,
    };
    let nc = gr.node_count();
    gr.link_edges().map_err(|i| invalid_node_err(i.index(), nc))?;
    Ok(gr)
}

impl<'de, N, E, Ty, Ix> Deserialize<'de> for Graph<N, E, Ty, Ix>
    where Ty: EdgeType,
          Ix: IndexType + Deserialize<'de>,
          N: Deserialize<'de>,
          E: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        Self::from_deserialized(DeserGraph::deserialize(deserializer)?)
    }
}

