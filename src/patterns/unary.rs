use std::any::Any;
use std::sync::mpsc::Sender;
use std::thread;

use crate::{
    channels::{
        BidirChannel, RecvChannel, SendChannel, StrippedBidirChannel, StrippedRecvChannel,
        StrippedSendChannel,
    },
    function_transforms,
    types::{functions, ids, JoinPattern, Message, Packet},
};

/**********************************
 * Send Join Pattern Construction *
 **********************************/

/// `SendChannel` partial Join Pattern.
pub struct SendPartialPattern<T> {
    junction_id: ids::JunctionId,
    send_channel: StrippedSendChannel<T>,
    sender: Sender<Packet>,
}

impl<T> SendPartialPattern<T>
where
    T: Any + Send,
{
    pub(crate) fn new(
        junction_id: ids::JunctionId,
        send_channel: StrippedSendChannel<T>,
        sender: Sender<Packet>,
    ) -> SendPartialPattern<T> {
        SendPartialPattern {
            junction_id,
            send_channel,
            sender,
        }
    }

    /// Create a binary partial Join Pattern with two send channels.
    ///
    /// Create a new binary partial Join Pattern that starts with the current
    /// pattern and includes a new `SendChannel` after that.
    ///
    /// # Panics
    ///
    /// Panics if the supplied `SendChannel` does not carry the same
    /// `JunctionID` as this `SendPartialPattern`, i.e. has not been created by
    /// and is associated with the same `Junction`.
    pub fn and<U>(self, send_channel: &SendChannel<U>) -> super::binary::SendPartialPattern<T, U>
    where
        U: Any + Send,
    {
        if send_channel.junction_id() == self.junction_id {
            super::binary::SendPartialPattern::new(
                self.junction_id,
                self.send_channel,
                send_channel.strip(),
                self.sender,
            )
        } else {
            panic!(
                "SendChannel and SendPartialPattern not associated \
                    with same Junction! Please use a SendChannel created \
                    using the same Junction as this partially complete Join \
                    Pattern"
            );
        }
    }

    /// Create a binary partial Join Pattern with a send and receive channel.
    ///
    /// Create a new binary partial Join Pattern that starts with the current
    /// pattern and includes a new `RecvChannel` after that.
    ///
    /// # Panics
    ///
    /// Panics if the supplied `RecvChannel` does not carry the same
    /// `JunctionID` as this `SendPartialPattern`, i.e. has not been created by
    /// and is associated with the same `Junction`.
    pub fn and_recv<R>(
        self,
        recv_channel: &RecvChannel<R>,
    ) -> super::binary::RecvPartialPattern<T, R>
    where
        R: Any + Send,
    {
        if recv_channel.junction_id() == self.junction_id {
            super::binary::RecvPartialPattern::new(
                self.send_channel,
                recv_channel.strip(),
                self.sender,
            )
        } else {
            panic!(
                "RecvChannel and SendPartialPattern not associated \
                    with same Junction! Please use a RecvChannel created \
                    using the same Junction as this partially complete Join \
                    Pattern"
            );
        }
    }

    /// Create a binary partial Join Pattern with a send and bidirectional channel.
    ///
    /// Create a new binary partial Join Pattern that starts with the current
    /// pattern and includes a new `BidirChannel` after that.
    ///
    /// # Panics
    ///
    /// Panics if the supplied `BidirChannel` does not carry the same
    /// `JunctionID` as this `SendPartialPattern`, i.e. has not been created by
    /// and is associated with the same `Junction`.
    pub fn and_bidir<U, R>(
        self,
        bidir_channel: &BidirChannel<U, R>,
    ) -> super::binary::BidirPartialPattern<T, U, R>
    where
        U: Any + Send,
        R: Any + Send,
    {
        if bidir_channel.junction_id() == self.junction_id {
            super::binary::BidirPartialPattern::new(
                self.send_channel,
                bidir_channel.strip(),
                self.sender,
            )
        } else {
            panic!(
                "BidirChannel and SendPartialPattern not associated \
                    with same Junction! Please use a BidirChannel created \
                    using the same Junction as this partially complete Join \
                    Pattern"
            );
        }
    }

    /// Create full Join Pattern and send request to add it to `Junction`.
    ///
    /// Create a full Join Pattern by taking the channels that are part of
    /// the partial pattern and adding a function to be executed when there
    /// is at least one message sent on each channel. Attempt to add the
    /// Join Pattern to the `Junction` after creation.
    ///
    /// # Panics
    ///
    /// Panics if it was not possible to send the request to add the newly
    /// create Join Pattern to the `Junction`.
    pub fn then_do<F>(self, f: F)
    where
        F: Fn(T) -> () + Send + Clone + 'static,
    {
        let join_pattern = JoinPattern::UnarySend(SendJoinPattern::new(
            self.send_channel.id(),
            function_transforms::unary::transform_send(f),
        ));

        self.sender
            .send(Packet::AddJoinPatternRequest { join_pattern })
            .unwrap();
    }
}

/// `SendChannel` full Join Pattern.
pub struct SendJoinPattern {
    channel_id: ids::ChannelId,
    f: functions::unary::FnBox,
}

impl SendJoinPattern {
    pub(crate) fn new(channel_id: ids::ChannelId, f: functions::unary::FnBox) -> SendJoinPattern {
        SendJoinPattern { channel_id, f }
    }

    pub(crate) fn channels(&self) -> Vec<ids::ChannelId> {
        vec![self.channel_id]
    }

    /// Return the ID of the channel in this Join Pattern.
    pub(crate) fn channel_id(&self) -> ids::ChannelId {
        self.channel_id
    }

    /// Fire Join Pattern by running associated function in separate thread.
    pub(crate) fn fire(&self, arg: Message) {
        let f_clone = self.f.clone();

        thread::spawn(move || {
            (*f_clone)(arg);
        });
    }
}

/*************************************
 * Receive Join Pattern Construction *
 *************************************/

/// `RecvChannel` partial Join Pattern.
pub struct RecvPartialPattern<R> {
    recv_channel: StrippedRecvChannel<R>,
    sender: Sender<Packet>,
}

impl<R> RecvPartialPattern<R>
where
    R: Any + Send,
{
    pub(crate) fn new(
        recv_channel: StrippedRecvChannel<R>,
        sender: Sender<Packet>,
    ) -> RecvPartialPattern<R> {
        RecvPartialPattern {
            recv_channel,
            sender,
        }
    }

    /// Create full Join Pattern and send request to add it to `Junction`.
    ///
    /// Create a full Join Pattern by taking the channels that are part of
    /// the partial pattern and adding a function to be executed when there
    /// is at least one message sent on each channel. Attempt to add the
    /// Join Pattern to the `Junction` after creation.
    ///
    /// # Panics
    ///
    /// Panics if it was not possible to send the request to add the newly
    /// create Join Pattern to the `Junction`.
    pub fn then_do<F>(self, f: F)
    where
        F: Fn() -> R + Send + Clone + 'static,
    {
        let join_pattern = JoinPattern::UnaryRecv(RecvJoinPattern::new(
            self.recv_channel.id(),
            function_transforms::unary::transform_recv(f),
        ));

        self.sender
            .send(Packet::AddJoinPatternRequest { join_pattern })
            .unwrap();
    }
}

/// `RecvChannel` full Join Pattern.
///
/// N.B.: While this struct appears to be a duplicate of `SendJoinPattern`
/// in terms of code, it is used to distinguish the capability of the
/// Join Pattern within the `Junction` through its type.
pub struct RecvJoinPattern {
    channel_id: ids::ChannelId,
    f: functions::unary::FnBox,
}

impl RecvJoinPattern {
    pub(crate) fn new(channel_id: ids::ChannelId, f: functions::unary::FnBox) -> RecvJoinPattern {
        RecvJoinPattern { channel_id, f }
    }

    pub(crate) fn channels(&self) -> Vec<ids::ChannelId> {
        vec![self.channel_id]
    }

    /// Return the ID of the channel in this Join Pattern.
    pub(crate) fn channel_id(&self) -> ids::ChannelId {
        self.channel_id
    }

    /// Fire Join Pattern by running associated function in separate thread.
    pub(crate) fn fire(&self, return_sender: Message) {
        let f_clone = self.f.clone();

        thread::spawn(move || {
            (*f_clone)(return_sender);
        });
    }
}

/*******************************************
 * Bidirectional Join Pattern Construction *
 *******************************************/

/// Bidirectional channel partial Join Pattern.
pub struct BidirPartialPattern<T, R> {
    bidir_channel: StrippedBidirChannel<T, R>,
    sender: Sender<Packet>,
}

impl<T, R> BidirPartialPattern<T, R>
where
    T: Any + Send,
    R: Any + Send,
{
    pub(crate) fn new(
        bidir_channel: StrippedBidirChannel<T, R>,
        sender: Sender<Packet>,
    ) -> BidirPartialPattern<T, R> {
        BidirPartialPattern {
            bidir_channel,
            sender,
        }
    }

    /// Create full Join Pattern and send request to add it to `Junction`.
    ///
    /// Create a full Join Pattern by taking the channels that are part of
    /// the partial pattern and adding a function to be executed when there
    /// is at least one message sent on each channel. Attempt to add the
    /// Join Pattern to the `Junction` after creation.
    ///
    /// # Panics
    ///
    /// Panics if it was not possible to send the request to add the newly
    /// create Join Pattern to the `Junction`.
    pub fn then_do<F>(self, f: F)
    where
        F: Fn(T) -> R + Send + Clone + 'static,
    {
        let join_pattern = JoinPattern::UnaryBidir(BidirJoinPattern::new(
            self.bidir_channel.id(),
            function_transforms::unary::transform_bidir(f),
        ));

        self.sender
            .send(Packet::AddJoinPatternRequest { join_pattern })
            .unwrap();
    }
}

/// `BidirChannel` full Join Pattern.
pub struct BidirJoinPattern {
    channel_id: ids::ChannelId,
    f: functions::unary::FnBox,
}

impl BidirJoinPattern {
    pub(crate) fn new(channel_id: ids::ChannelId, f: functions::unary::FnBox) -> BidirJoinPattern {
        BidirJoinPattern { channel_id, f }
    }

    pub(crate) fn channels(&self) -> Vec<ids::ChannelId> {
        vec![self.channel_id]
    }

    /// Return the ID of the channel in this Join Pattern.
    pub(crate) fn channel_id(&self) -> ids::ChannelId {
        self.channel_id
    }

    /// Fire Join Pattern by running associated function in separate thread.
    pub(crate) fn fire(&self, arg_and_sender: Message) {
        let f_clone = self.f.clone();

        thread::spawn(move || {
            (*f_clone)(arg_and_sender);
        });
    }
}
