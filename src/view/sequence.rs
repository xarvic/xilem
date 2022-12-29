use std::any::Any;
use std::ops::Range;
use either::Either;
use crate::event::EventResult;
use crate::id::Id;
use crate::View;
use crate::view::Cx;
use crate::view::sequence::Position::Last;
use crate::widget::{ChangeFlags, Pod};

/// A sequence on view nodes.
///
/// This is one of the central traits for representing UI. Every view which has a collection of
/// children uses an instance of this trait to specify them.
///
/// The framework will then run methods on these views to create the associated
/// state tree and widget tree, as well as incremental updates and event
/// propagation. The methods in the `ViewSequence` trait correspond to the ones in the `View` trait.
///
/// The `View` trait is parameterized by `T`, which is known as the "app state",
/// and also a type for actions which are passed up the tree in event
/// propagation. During event handling, mutable access to the app state is
/// given to view nodes, which in turn can make expose it to callbacks.
pub trait ViewSequence<T, M, A = ()>: Send {
    /// Associated states for the views.
    type State: Send;

    /// A stable index of the views. The mapping of Index => View should be consitent even after the
    /// view was removed and added again or after new views are added to to collection.
    type Index: Clone;

    /// Build the associated widgets and initialize all states.
    fn build(&self, cx: &mut Cx) -> (Self::State, Vec<(Pod, M)>);

    /// Update the associated widget.
    ///
    /// Returns `true` when anything has changed.
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        state: &mut Self::State,
        element: &mut Vec<(Pod, M)>,
        offset: usize,
    ) -> ChangeFlags;

    /// Propagate an event.
    ///
    /// Handle an event, propagating to children if needed. Here, `id_path` is a slice
    /// of ids beginning at a child of this view.
    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> EventResult<A>;

    /// Returns the stable Id of a given element of the sequence. This index does not change even if
    /// elements are added or removed from the sequence.
    ///
    /// # Panics
    /// Panics if `position >= self.count()`.
    ///
    fn to_stable(&self, state: &Self::State, position: Position) -> Self::Index;

    /// Returns the current position of a stable index in this sequence. If the returned bool is false
    /// the index is not inside the sequence. In this case the returned usize points to the following
    /// element in the sequence.
    ///
    /// The value of the usize is in the range of `0..=self.count()`.
    /// A return value of `(0, false)` means the value is
    fn from_stable(&self, state: &Self::State, index: Self::Index) -> (usize, bool);

    /// The amount of view-element pairs currently managed by this sequence.
    ///
    /// This means that the value of this method can change after calling build or rebuild.
    fn count(&self, state: &Self::State) -> usize;

    /// This methods tells the Sequence which elements are needed, to fill the visible area with
    /// views.
    ///
    /// The value is an upperbound estimate of Layout-Widget displaying the sequence. The Sequence
    /// should try to only load as much views as requested by this method. It can inteanlly store
    /// more but should only expose the needed views to the consumer.
    fn request(&self, state: &mut Self::State, index: Self::Index, offset: Range<isize>) -> Range<isize>;
}

struct Concat<U, V>(U, V);

impl<T, A, M, U: ViewSequence<T, M, A>, V: ViewSequence<T, M, A>> ViewSequence<T, M, A> for Concat<U, V> {
    type State = (U::State, V::State);
    type Index = Either<U::Index, V::Index>;

    fn build(&self, cx: &mut Cx) -> (Self::State, Vec<(Pod, M)>) {
        let (state_u, mut elements_u) = self.0.build(cx);
        let (state_v, mut elements_v) = self.1.build(cx);
        elements_u.append(&mut elements_v);

        ((state_u, state_v), elements_u)
    }

    fn rebuild(&self, cx: &mut Cx, prev: &Self, state: &mut Self::State, element: &mut Vec<(Pod, M)>, offset: usize) -> ChangeFlags {
        let flags = self.0.rebuild(cx, &prev.0, &mut state.0, elements, 0);
        let offset = self.0.count(&state.0);
        let flags2 = self.1.rebuild(cx, &prev.1, &mut state.1, elements, offset);
        flags.union(flags2)
    }

    fn event(&self, id_path: &[Id], state: &mut Self::State, event: Box<dyn Any>, app_state: &mut T) -> EventResult<A> {
        self.0.event(id_path, &mut state.0, event, app_state)
            .or(|event|{
                self.1.event(id_path, &mut state.1, event, app_state)
            })
    }

    fn to_stable(&self, state: &Self::State, position: Position) -> Self::Index {
        match position {
            Position::First => self.0.to_stable(&state.0, Position::First),
            Position::Sequence(position) => {
                let first_length = self.0.count( &state.0);
                if position < first_length {
                    self.0.to_stable(&state.0, Position::Sequence(position))
                } else {
                    self.1.to_stable(&state.1, Position::Sequence(position - first_length))
                }
            },
            Position::Last => self.1.to_stable(&state.1, Position::Last)
        }
    }

    fn from_stable(&self, state: &Self::State, index: Self::Index) -> (usize, bool) {
        match index {
            Either::Left(index) => {
                self.0.from_stable(&state.0, index)
            }
            Either::Right(index) => {
                let first_len = self.0.count(&state.0);
                let (pos, hit) = self.1.from_stable(&state.1, index);
                (first_len + pos, hit)
            }
        }
    }

    fn count(&self, state: &Self::State) -> usize {
        self.0.count(&state.0) + self.1.count(&state.1)
    }

    fn request(&self, state: &mut State, index: Self::Index, offset: Range<isize>) {
        match index {
            Either::Left(index) => {
                let right = self.0.request(&mut state.0, index.clone(), offset).end;
                let rq = (offset.end - right).max(0);
                let index = self.1.to_stable(&state.1, Position::First);
                self.1.request(&mut state.1, index, 0..rq);
            }
            Either::Right(index) => {
                let left = self.1.request(&mut state.1, index.clone(), offset).start;
                let rq = (offset.start - left).min(0);
                let index = self.1.to_stable(&state.1, Position::Last);
                self.1.request(&mut state.1, index, rq..0);
            }
        }
    }
}

pub trait ViewEntry<T, M: Clone, A> {
    type Entry: View<T, A>;

    fn view(&self) -> &Self::Entry;

    fn meta(&self) -> M;
}

impl<V: View<T, A>, T, M: Clone + Default, A> ViewEntry<T, M, A> for V {
    type Entry = V;

    fn view(&self) -> &Self::Entry {
        self
    }

    fn meta(&self) -> M {
        M::default()
    }
}

impl<V: View<T, A>, T, M: Clone, A> ViewEntry<T, M, A> for (V, M) {
    type Entry = V;

    fn view(&self) -> &Self::Entry {
        &self.0
    }

    fn meta(&self) -> M {
        self.1.clone()
    }
}

enum Position {
    First,
    Sequence(usize),
    Last,
}

trait ViewList<T, A = ()> {

}



macro_rules! impl_view_tuple {
    ( $n: tt; $( $t:ident),* ; $( $i:tt ),* ) => {
        impl<T, M, A, $( $t: ViewEntry<T, M, A> ),* > ViewSequence<T, M, A> for ( $( $t, )* )
            where $( <$t as View<T, A>>::Element: 'static ),*
        {
            type State = ( $( $t::State, )* [Id; $n]);

            type Index = u8;

            fn build(&self, cx: &mut Cx) -> (Self::State, Vec<(Pod, M)>) {
                let b = ( $( self.$i.view().build(cx), )* );
                let state = ( $( b.$i.1, )* [ $( b.$i.0 ),* ]);
                let els = vec![ $( (Pod::new(b.$i.2), ) ),* ];
                (state, els)
            }

            fn rebuild(
                &self,
                cx: &mut Cx,
                prev: &Self,
                state: &mut Self::State,
                els: &mut Vec<Pod>,
            ) -> bool {
                let mut changed = false;
                $(
                if self.$i
                    .rebuild(cx, &prev.$i, &mut state.$n[$i], &mut state.$i,
                        els[$i].downcast_mut().unwrap())
                {
                    els[$i].request_update();
                    changed = true;
                }
                )*
                changed
            }

            fn event(
                &self,
                id_path: &[Id],
                state: &mut Self::State,
                event: Box<dyn Any>,
                app_state: &mut T,
            ) -> EventResult<A> {
                let hd = id_path[0];
                let tl = &id_path[1..];
                $(
                if hd == state.$n[$i] {
                    self.$i.event(tl, &mut state.$i, event, app_state)
                } else )* {
                    crate::event::EventResult::Stale
                }
            }

            fn to_stable(&self, state: &Self::State, position: Position) -> Self::Index {
                match position {
                    Position::First => 0,
                    Position::Sequence(n) =>  {
                        n.try_into().unwrap()
                    },
                    POsition::Last => $n - 1,
                }
            }

            fn from_stable(&self, state: &Self::State, index: Self::Index) -> (usize, bool) {
                (index as _, true)
            }

            fn count(&self, state: &Self::State) -> usize {
                $n
            }

            fn request(&self, state: &mut Self::State, index: Self::Index, offset: Range<isize>) -> Range<isize> {
                let i = index as _;
                -i..($n-i-1)
            }
        }
    }
}

impl_view_tuple!(1; V0; 0);
impl_view_tuple!(2; V0, V1; 0, 1);
impl_view_tuple!(3; V0, V1, V2; 0, 1, 2);
impl_view_tuple!(4; V0, V1, V2, V3; 0, 1, 2, 3);
impl_view_tuple!(5; V0, V1, V2, V3, V4; 0, 1, 2, 3, 4);
impl_view_tuple!(6; V0, V1, V2, V3, V4, V5; 0, 1, 2, 3, 4, 5);
impl_view_tuple!(7; V0, V1, V2, V3, V4, V5, V6; 0, 1, 2, 3, 4, 5, 6);
impl_view_tuple!(8;
    V0, V1, V2, V3, V4, V5, V6, V7;
    0, 1, 2, 3, 4, 5, 6, 7
);
impl_view_tuple!(9;
    V0, V1, V2, V3, V4, V5, V6, V7, V8;
    0, 1, 2, 3, 4, 5, 6, 7, 8
);
impl_view_tuple!(10;
    V0, V1, V2, V3, V4, V5, V6, V7, V8, V9;
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9
);