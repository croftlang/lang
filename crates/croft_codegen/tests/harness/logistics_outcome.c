/* Runs Logistics for the test target, then finds the sample cargo and its
   last handling event, so the test can check the journey played out as
   scripted. */
#include <stdio.h>
#include "program.h"

int croft_program_main(int argc, char **argv);

int main(int argc, char **argv)
{
    int status = croft_program_main(argc, argv);
    croft_result_Shipping_Cargo_error cargo;
    croft_result_Shipping_HandlingEvent_error last;
    Shipping_Delivery *delivery = &cargo.value.delivery;
    Shipping_Cargos_find(1, &cargo);
    Shipping_HandlingEvents_find(6, &last);
    printf("status %d\n", status);
    printf("legs %u\n", (unsigned) cargo.value.itinerary.legs.length);
    printf("claimed %d at %.*s\n",
           delivery->transport_status == Shipping_TransportStatus_Claimed,
           (int) delivery->last_location.value.value.length,
           delivery->last_location.value.value.text);
    printf("misdirected %d\n", (int) delivery->misdirected);
    printf("last event %d on day %u\n", (int) last.ok,
           (unsigned) last.value.completed_on.value);
    return 0;
}
